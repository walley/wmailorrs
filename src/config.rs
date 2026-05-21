use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProfile {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub tls: bool,
}

pub fn config_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "wmailor")
        .context("could not resolve config directory")?;
    let dir = dirs.config_dir().to_path_buf();
    fs::create_dir_all(&dir)?;
    fs::create_dir_all(dir.join("connections"))?;
    Ok(dir)
}

pub fn connections_dir() -> Result<PathBuf> {
    Ok(config_dir()?.join("connections"))
}

pub fn theme_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("theme.json"))
}

pub fn save_connection(profile: &ConnectionProfile) -> Result<()> {
    let path = connections_dir()?.join(format!("{}.json", sanitize_name(&profile.name)));
    let data = serde_json::to_string_pretty(profile)?;
    fs::write(path, data)?;
    Ok(())
}

pub fn load_connection(name: &str) -> Result<ConnectionProfile> {
    let path = connections_dir()?.join(format!("{}.json", sanitize_name(name)));
    let data = fs::read_to_string(&path).with_context(|| format!("missing profile: {name}"))?;
    Ok(serde_json::from_str(&data)?)
}

pub fn list_connections() -> Result<Vec<String>> {
    let dir = connections_dir()?;
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

pub fn save_theme(theme: &crate::theme::Theme) -> Result<()> {
    let data = serde_json::to_string_pretty(theme)?;
    fs::write(theme_path()?, data)?;
    Ok(())
}

pub fn load_theme() -> crate::theme::Theme {
    theme_path()
        .ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn download_dir() -> Result<PathBuf> {
    let dir = config_dir()?.join("downloads");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}
