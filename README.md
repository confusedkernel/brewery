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
- **🍻 Cask Browser** — Toggle to installed casks and manage them with the same quick actions
- **🔍 Advanced Search** — Search all available Homebrew packages
- **📊 Rich Details** — View descriptions, homepage, versions, dependencies, and reverse dependencies
- **📈 Status Panel** — Track activity, diagnostics issues, outdated packages, and brew update recency
- **🧰 Service Controls** — View Homebrew service states and run start/stop/restart from the Services tab
- **📏 Size Analytics** — Leaderboard of installed packages by disk usage
- **⚡ Quick Actions** — Install, uninstall, upgrade, upgrade all outdated, cleanup, autoremove, and export Brewfiles
- **🔄 Self Update** — Detect new Brewery releases and update in-app via Cargo
- **🎯 Outdated Workflow** — Toggle outdated-only leaves filter and batch-upgrade outdated packages
- **🕘 Command History** — Browse recent command runs with exit status in Status -> History
- **🔁 Background Refresh** — Periodically refresh status/outdated/services and show in-panel progress
- **🖱️ Mouse Navigation** — Click to focus/select and use scroll wheel to navigate panels
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
| `Shift+C`          | Toggle formulae/casks list |
| `l`/`;` or `←`/`→` | Cycle status tabs          |

### Mouse

| Input          | Action                                                  |
| -------------- | ------------------------------------------------------- |
| Left click     | Focus panel, select list item, or switch status tab    |
| Scroll wheel   | Scroll/select within the panel under the cursor        |

### Search

| Key     | Action                                |
| ------- | ------------------------------------- |
| `/`     | Filter installed list (formulae/casks) |
| `f`     | Search all packages                   |
| `Enter` | Confirm search / Exit filter mode     |
| `Esc`   | Cancel / Clear filter                 |

### Package Management

| Key     | Action                                     |
| ------- | ------------------------------------------ |
| `i`     | Install selected formula/cask (press twice to confirm)   |
| `u`     | Uninstall selected formula/cask (press twice to confirm) |
| `Shift+U` | Upgrade selected formula/cask, or upgrade all outdated formulae in Status -> Outdated (press twice to confirm) |
| `Enter` | Load package details                       |
| `d`     | Load dependencies and reverse dependencies |

### Maintenance

| Key | Action                         |
| --- | ------------------------------ |
| `r` | Refresh package list           |
| `s` | Load package sizes             |
| `h` | Run status check               |
| `Shift+P` | Update Brewery via Cargo (press twice to confirm) |
| `o` | Toggle outdated-only formula filter |
| `c` | Cleanup old versions           |
| `a` | Autoremove unused dependencies |
| `b` | Export Brewfile (bundle dump)  |
| `Shift+S` | Start selected service (Status -> Services, press twice to confirm) |
| `Shift+X` | Stop selected service (Status -> Services, press twice to confirm) |
| `Shift+R` | Restart selected service (Status -> Services, press twice to confirm) |

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
