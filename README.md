<div align="center">

# Brewery 🍺

**A blazingly fast terminal UI for Homebrew**

_Browse, search, and manage your Homebrew packages with ease_

[![Crates.io](https://img.shields.io/crates/v/brewery.svg)](https://crates.io/crates/brewery)
[![Downloads](https://img.shields.io/crates/d/brewery.svg)](https://crates.io/crates/brewery)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)
![Brewed Fresh](https://img.shields.io/badge/brewed-Fresh%20🍺-yellow?style=flat)
![Blazingly Fast](https://img.shields.io/badge/speed-blazingly%20⚡-brightgreen?style=flat)

---

</div>

## Features

- **📦 Package Browser** — Browse installed leaves with instant search filtering
- **🔍 Advanced Search** — Search all available Homebrew packages
- **📊 Rich Details** — View descriptions, homepage, versions, dependencies, and reverse dependencies
- **🏥 Health Dashboard** — Run diagnostics, check outdated packages, and monitor activity
- **📏 Size Analytics** — Leaderboard of installed packages by disk usage
- **⚡ Quick Actions** — Install, uninstall, cleanup, autoremove, and export Brewfiles
- **🎨 Adaptive Theming** — Auto-detects system theme with manual override (light/dark)
- **🖥️ Pure Terminal** — No browser required, works entirely in your terminal

## Installation

```bash
cargo install brewery
```

### Requirements

- Homebrew installed and available as `brew`
- Rust toolchain (edition 2024)
- Terminal with True Color support

### Font

Nerd Font is optional. Use ASCII mode with `BREWERY_ASCII=1` or press `Alt+i` in-app.

## Keyboard Shortcuts

### Navigation

| Key                | Action                     |
| ------------------ | -------------------------- |
| `j`/`k` or `↑`/`↓` | Move selection             |
| `Tab`/`Shift+Tab`  | Cycle focus between panels |

### Search

| Key     | Action                                |
| ------- | ------------------------------------- |
| `/`     | Filter installed leaves (live filter) |
| `f`     | Search all packages                   |
| `Enter` | Confirm search / Exit filter mode     |
| `Esc`   | Cancel / Clear filter                 |

### Package Management

| Key     | Action                                     |
| ------- | ------------------------------------------ |
| `i`     | Install package (press twice to confirm)   |
| `u`     | Uninstall package (press twice to confirm) |
| `Enter` | Load package details                       |
| `d`     | Load dependencies and reverse dependencies |

### Maintenance

| Key | Action                         |
| --- | ------------------------------ |
| `r` | Refresh package list           |
| `s` | Load package sizes             |
| `h` | Run health check               |
| `c` | Cleanup old versions           |
| `a` | Autoremove unused dependencies |
| `b` | Export Brewfile (bundle dump)  |

### View

| Key     | Action                         |
| ------- | ------------------------------ |
| `v`     | Toggle details/results view    |
| `t`     | Toggle theme (auto/light/dark) |
| `Alt+i` | Toggle Nerd Font / ASCII icons |
| `?`     | Show help                      |
| `q`     | Quit                           |

---

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for detailed release notes and version history.
