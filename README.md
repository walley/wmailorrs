# wmailor

Admin-oriented, text-based IMAP client for inspecting mail **as raw source** — no HTML rendering, no MIME “preview” chrome.

Built with [ratatui](https://ratatui.rs/). Layout inspired by Midnight Commander; menus inspired by Turbo Vision.

## Layout

```
┌─────────────┬──────────────────────────┐
│  Folders    │  Messages                │
│  (IMAP)     ├──────────────────────────┤
│             │  Message source / MIME   │
└─────────────┴──────────────────────────┘
│ F1 Help  F2 Menu ...          keybar   │
└────────────────────────────────────────┘
```

- **Left**: IMAP folder tree
- **Right top**: message list for the selected folder
- **Right bottom**: full RFC822 source with syntax coloring (headers, Received chains, X-* headers, etc.)
- MIME parts are a **foldable tree**; binary parts open in a hex view

## Features

- Connect / disconnect; save & load connection profiles (`~/.config/wmailor/`)
- Raw source display only — toggle **original** vs **decoded** body per MIME part
- Download MIME parts to disk
- Customizable color scheme (menu → Colors)
- Hex dump for binary attachments (`address 00 11 DD … safe text`)

## Build

```bash
cargo build --release
```

## Run

```bash
cargo run --release
```

## Keys (default)

| Key | Action |
|-----|--------|
| Tab | Cycle focus (folders → messages → content) |
| ↑/↓ | Move selection |
| Enter | Open folder / fetch message |
| Space | Toggle MIME fold |
| `o` | Toggle original/decoded for focused MIME part |
| `x` | Hex view for binary part |
| `d` | Download focused MIME part |
| F2 / Alt | Menu |
| F10 | Quit |
| `/` | Filter messages (when messages focused) |

Connection profiles are JSON files under `~/.config/wmailor/connections/`.

## Disclaimer

This is an **admin/debug** tool. It assumes you trust the server and your terminal. Do not use it as a daily MUA for untrusted mail without understanding the risks of displaying raw source.
