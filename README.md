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

## Why Brewery

Homebrew tells you *what* you have. Brewery tells you why it's there and what it
costs you — questions the CLI can't answer without chaining several commands and
reading the output yourself.

**"Why do I have this?"** — select any installed formula and the Details panel
traces it back to whatever you actually asked for:

```
  openssl@3

  Required by
    openssl@3 ← nmap
    12 formulae depend on it directly
```

There is no `brew why`. Getting this by hand means `brew uses --installed`, then
tracing each result upward one level at a time.

**Uninstall impact** — `brew uninstall` removes exactly what you name and
silently strands its dependencies until you remember `brew autoremove`. Brewery
tells you first:

```
Uninstall formula ffmpeg? +46 orphans (~263.1M) [u] confirm, [Esc] cancel
```

While the confirmation is armed, the Details panel lists all 46 by name, so you
can check what you are about to lose before pressing `u` again.

**Autoremove preview** — `brew autoremove` deletes on the first keypress.
Brewery arms it the same way as an uninstall, with the whole orphan set and the
disk it frees listed in Details before you confirm:

```
Autoremove 12 orphans (~301.4M)? [a] confirm, [Esc] cancel
```

All three are built from Homebrew's own install receipts — the same data
`brew autoremove` reasons about — so the orphan set is verified to agree with
`brew autoremove --dry-run` exactly. Notably, formulae you installed on purpose
are never reported as collateral, even when something else also depends on them.

## Features

**Browsing**

- Installed formulae, either leaves only or all of them (`L`)
- Installed casks, with the same actions (`Shift+C`)
- Sort by name, disk size, or install date (`Shift+O`)
- Instant search filtering, plus search across all of Homebrew
- Rich details: description, homepage, versions, dependencies, reverse dependencies
- Open a package's homepage in your browser (`g`)

**Insight**

- Dependency provenance — why each formula is installed, and when
- Uninstall impact preview with orphan count and disk reclaimed
- Autoremove preview: what `brew autoremove` would delete, before it does
- Outdated packages with the version jump each upgrade makes, formulae and casks alike
- Size leaderboard of installed packages by disk usage
- Status panel for diagnostics, outdated packages, and `brew update` recency

**Management**

- Install, uninstall, upgrade, and batch-upgrade all outdated packages
- Pin and unpin formulae, with pinned ones marked in the list and the Outdated tab
- `brew update`, cleanup, autoremove, and Brewfile export
- Service controls — start, stop, restart, inspect, and filter `brew services`
- Command history with exit status
- Self-update via Cargo when a new release is available

**Interface**

- Adaptive theming with light/dark auto-detection and manual override
- Mouse navigation — click to focus and select, scroll to move
- Nerd Font icons with an ASCII fallback
- Background refresh with in-panel progress
- Runs entirely in your terminal

## Installation

```bash
cargo install brewery
```

### Requirements

- Homebrew installed and available as `brew`
- Rust toolchain (edition 2024)
- Terminal with True Color support

### Environment

| Variable         | Effect                                             |
| ---------------- | -------------------------------------------------- |
| `BREWERY_ASCII`  | Set to `1` to force ASCII icons instead of Nerd Font |
| `BREWERY_MOUSE`  | Set to `0` to start with mouse capture disabled     |

Both are also toggleable in-app with `Alt+i` and `m`.

## Keyboard Shortcuts

Press `?` in-app for the same list, where `Enter` runs the highlighted command.

### Navigation

| Key                | Action                     |
| ------------------ | -------------------------- |
| `j`/`k` or `↑`/`↓` | Move selection             |
| `Tab`/`Shift+Tab`  | Cycle focus between panels |
| `l`/`;` or `←`/`→` | Cycle status tabs          |

### Search

| Key       | Action                                   |
| --------- | ---------------------------------------- |
| `/`       | Filter the installed list                |
| `f`       | Search all Homebrew packages             |
| `Shift+C` | Toggle formulae / casks                  |
| `Shift+L` | Toggle leaves only / all installed formulae |
| `Shift+O` | Cycle sort: name / size / recently installed |
| `Enter`   | Confirm search, or exit filter mode      |
| `Esc`     | Cancel, or clear the filter              |

### Actions

Destructive actions are confirmed by pressing the same key twice.

| Key       | Action                                                                   |
| --------- | ------------------------------------------------------------------------ |
| `Enter`   | Load package details                                                     |
| `d`       | Load dependencies and reverse dependencies                               |
| `i`       | Install selected package                                                 |
| `u`       | Uninstall selected package, with orphan impact preview                   |
| `Shift+U` | Upgrade selected package, or all outdated from Status → Outdated         |
| `p`       | Pin or unpin selected formula                                            |
| `g`       | Open selected package's homepage in the browser                          |
| `o`       | Toggle outdated-only filter                                              |
| `Shift+P` | Update Brewery via Cargo                                                 |

### Services

Available from the Status → Services tab.

| Key       | Action                                    |
| --------- | ----------------------------------------- |
| `Shift+S` | Start selected service                    |
| `Shift+X` | Stop selected service                     |
| `Shift+R` | Restart selected service                  |
| `Shift+I` | Show service info (`brew services info`)  |
| `Shift+F` | Filter to failed services                 |
| `Shift+A` | Filter to auto-start services             |
| `Shift+K` | Cycle backend filter (all/formula/cask)   |

### Data & Maintenance

| Key | Action                         |
| --- | ------------------------------ |
| `r` | Refresh formulae and casks     |
| `s` | Load package sizes             |
| `h` | Run status check               |
| `e` | Run `brew update`              |
| `c` | Cleanup old versions           |
| `a` | Autoremove unused dependencies, with preview |
| `b` | Export Brewfile (bundle dump)  |

### View

| Key     | Action                          |
| ------- | ------------------------------- |
| `v`     | Toggle details / results view   |
| `t`     | Cycle theme (auto/light/dark)   |
| `m`     | Toggle mouse support            |
| `Alt+i` | Toggle Nerd Font / ASCII icons  |
| `?`     | Show help                       |
| `q`     | Quit                            |

### Mouse

| Input        | Action                                              |
| ------------ | --------------------------------------------------- |
| Left click   | Focus a panel, select an item, or switch status tab |
| Scroll wheel | Scroll or select within the panel under the cursor  |

---

## Notes

Dependency analysis covers formulae. Casks are browsed and managed the same way,
but Homebrew keeps no install receipts for them, so provenance and impact
previews are omitted rather than guessed at.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for detailed release notes and version history.
