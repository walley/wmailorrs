use crate::config::ConnectionProfile;
use crate::imap::types::{FetchedMessage, FolderEntry, MessageEntry};
use anyhow::{anyhow, Context, Result};
use imap::{Client, Session};
use native_tls::TlsConnector;
use std::net::TcpStream;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};

pub enum ImapCommand {
    Connect(ConnectionProfile),
    Disconnect,
    ListFolders,
    SelectFolder(String),
    ListMessages { offset: u32, limit: u32 },
    FetchMessage(u32),
    Quit,
}

pub enum ImapEvent {
    Connected(String),
    Disconnected,
    Folders(Vec<FolderEntry>),
    FolderSelected(String),
    Messages(Vec<MessageEntry>),
    MessageFetched(FetchedMessage),
    Error(String),
    Status(String),
}

pub struct ImapWorker {
    cmd_tx: Sender<ImapCommand>,
    join: Option<JoinHandle<()>>,
}

impl ImapWorker {
    pub fn spawn() -> (Self, Receiver<ImapEvent>) {
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
        let (evt_tx, evt_rx) = std::sync::mpsc::channel();

        let join = thread::spawn(move || worker_loop(cmd_rx, evt_tx));

        (
            Self {
                cmd_tx,
                join: Some(join),
            },
            evt_rx,
        )
    }

    pub fn send(&self, cmd: ImapCommand) {
        let _ = self.cmd_tx.send(cmd);
    }
}

impl Drop for ImapWorker {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(ImapCommand::Quit);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

struct WorkerState {
    session: Option<AnySession>,
    folder: Option<String>,
}

fn worker_loop(cmd_rx: Receiver<ImapCommand>, evt_tx: Sender<ImapEvent>) {
    let mut state = WorkerState {
        session: None,
        folder: None,
    };

    while let Ok(cmd) = cmd_rx.recv() {
        match cmd {
            ImapCommand::Quit => break,
            other => {
                if let Err(e) = handle_command(&mut state, other, &evt_tx) {
                    let _ = evt_tx.send(ImapEvent::Error(e.to_string()));
                }
            }
        }
    }
}

fn handle_command(state: &mut WorkerState, cmd: ImapCommand, evt_tx: &Sender<ImapEvent>) -> Result<()> {
    match cmd {
        ImapCommand::Connect(profile) => {
            let _ = evt_tx.send(ImapEvent::Status(format!(
                "Connecting to {}…",
                profile.host
            )));
            state.session = Some(connect(&profile)?);
            state.folder = None;
            let _ = evt_tx.send(ImapEvent::Connected(profile.name.clone()));
            let _ = evt_tx.send(ImapEvent::Status("Connected".into()));
        }
        ImapCommand::Disconnect => {
            state.session = None;
            state.folder = None;
            let _ = evt_tx.send(ImapEvent::Disconnected);
        }
        ImapCommand::ListFolders => {
            let session = state.session.as_mut().context("not connected")?;
            let boxes = session.list(Some(""), Some("*"))?;
            let mut folders = Vec::new();
            for entry in boxes.iter() {
                folders.push(FolderEntry {
                    name: entry.name().to_string(),
                    delimiter: entry.delimiter().and_then(|d| d.chars().next()),
                    attributes: entry
                        .attributes()
                        .iter()
                        .map(|a| format!("{a:?}"))
                        .collect(),
                });
            }
            folders.sort_by(|a, b| a.name.cmp(&b.name));
            let _ = evt_tx.send(ImapEvent::Folders(folders));
        }
        ImapCommand::SelectFolder(name) => {
            let session = state.session.as_mut().context("not connected")?;
            session.select(&name)?;
            state.folder = Some(name.clone());
            let _ = evt_tx.send(ImapEvent::FolderSelected(name));
        }
        ImapCommand::ListMessages { offset, limit } => {
            let session = state.session.as_mut().context("not connected")?;
            let folder = state.folder.as_ref().context("no folder selected")?;
            let total = session.select(folder)?.exists;
            if total == 0 {
                let _ = evt_tx.send(ImapEvent::Messages(vec![]));
                return Ok(());
            }
            let start = total.saturating_sub(offset).max(1);
            let end = start.saturating_sub(limit - 1).max(1);
            let seq_set = format!("{end}:{start}");
            let q = "(UID FLAGS RFC822.SIZE BODY.PEEK[HEADER.FIELDS (FROM SUBJECT DATE)])";
            let fetches = session.uid_fetch(&seq_set, q)?;
            let mut messages = Vec::new();
            for f in fetches.iter() {
                let uid = f.uid.ok_or_else(|| anyhow!("missing uid"))?;
                let size = f.size.unwrap_or(0);
                let header = f
                    .body()
                    .and_then(|b| std::str::from_utf8(b).ok())
                    .unwrap_or("");
                let summary = parse_header_summary(header);
                messages.push(MessageEntry {
                    uid,
                    seq: uid,
                    size,
                    summary,
                });
            }
            messages.sort_by(|a, b| b.uid.cmp(&a.uid));
            let _ = evt_tx.send(ImapEvent::Messages(messages));
        }
        ImapCommand::FetchMessage(uid) => {
            let session = state.session.as_mut().context("not connected")?;
            let q = "(RFC822)";
            let uid_set = uid.to_string();
            let fetches = session.uid_fetch(&uid_set, q)?;
            let f = fetches.iter().next().context("message not found")?;
            let raw = f.body().context("empty body")?;
            let raw = String::from_utf8_lossy(raw).into_owned();
            let _ = evt_tx.send(ImapEvent::MessageFetched(FetchedMessage { uid, raw }));
        }
        ImapCommand::Quit => {}
    }
    Ok(())
}

type TlsSession = Session<native_tls::TlsStream<TcpStream>>;
type PlainSession = Session<TcpStream>;

enum AnySession {
    Tls(TlsSession),
    Plain(PlainSession),
}

impl AnySession {
    fn list(
        &mut self,
        reference: Option<&str>,
        mask: Option<&str>,
    ) -> imap::Result<imap::types::ZeroCopy<Vec<imap::types::Name>>> {
        match self {
            AnySession::Tls(s) => s.list(reference, mask),
            AnySession::Plain(s) => s.list(reference, mask),
        }
    }

    fn select(&mut self, mailbox: &str) -> imap::Result<imap::types::Mailbox> {
        match self {
            AnySession::Tls(s) => s.select(mailbox),
            AnySession::Plain(s) => s.select(mailbox),
        }
    }

    fn uid_fetch(
        &mut self,
        uid_set: &str,
        query: &str,
    ) -> imap::Result<imap::types::ZeroCopy<Vec<imap::types::Fetch>>> {
        match self {
            AnySession::Tls(s) => s.uid_fetch(uid_set, query),
            AnySession::Plain(s) => s.uid_fetch(uid_set, query),
        }
    }
}

fn connect(profile: &ConnectionProfile) -> Result<AnySession> {
    if profile.tls {
        let tls = TlsConnector::builder().build()?;
        let client = imap::connect(
            (profile.host.as_str(), profile.port),
            profile.host.as_str(),
            &tls,
        )?;
        let session = client
            .login(&profile.user, &profile.password)
            .map_err(|(e, _)| e)?;
        Ok(AnySession::Tls(session))
    } else {
        let stream = TcpStream::connect((profile.host.as_str(), profile.port))?;
        let mut client = Client::new(stream);
        client.read_greeting()?;
        let session = client
            .login(&profile.user, &profile.password)
            .map_err(|(e, _)| e)?;
        Ok(AnySession::Plain(session))
    }
}

fn parse_header_summary(header: &str) -> String {
    let mut from = String::from("?");
    let mut subj = String::from("(no subject)");
    let mut date = String::new();
    for line in header.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("from:") {
            from = line.splitn(2, ':').nth(1).unwrap_or("?").trim().to_string();
        } else if lower.starts_with("subject:") {
            subj = line.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
        } else if lower.starts_with("date:") {
            date = line.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
        }
    }
    if date.is_empty() {
        format!("{from} — {subj}")
    } else {
        format!("{date} | {from} — {subj}")
    }
}
