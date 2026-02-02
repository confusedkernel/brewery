# Brewery

Brewery is a Rust TUI for Homebrew. WIP.

## Features
- Leaves browser with instant search filter
- Selection with details panel (description, homepage, installed versions)
- Optional deps and reverse deps loading
- Health panel (doctor, outdated leaves, activity)
- Size leaderboard for installed packages
- Package search, install, uninstall
- Cleanup, autoremove, and Brewfile export
- Light/dark theme with auto-detection and manual toggle

## Controls

### Navigation
- `j`/`k` or `↑`/`↓`: move selection
- `Tab`/`Shift+Tab`: cycle focus between panels

### Search Modes
- `/`: filter installed leaves (type to filter live)
  - `Enter`: exit filter mode and browse results (filter persists)
  - `Esc`: clear filter and exit
- `f`: search all packages (type query)
  - `Enter`: execute search (auto-enters results mode)
  - `Esc`: cancel and exit

### Package Actions
- `i`: install package (press twice to confirm)
- `u`: uninstall package (press twice to confirm)
- `Esc`: cancel pending action

### Details
- `Enter`: load details for selected package
- `d`: load deps and reverse deps for selected package

### Maintenance
- `r`: refresh leaves list
- `s`: load sizes
- `h`: health check
- `c`: cleanup
- `a`: autoremove
- `b`: bundle dump

### View
- `v`: toggle details/results view
- `t`: toggle theme (auto/light/dark)
- `Alt+i`: toggle Nerd/ASCII icons
- `?`: help
- `q`: quit

## Requirements
- Homebrew installed and available as `brew`
- Rust toolchain (edition 2024)

## Font
- Nerd Font is optional; set `BREWERY_ASCII=1` or press `Alt+i` to use ASCII icons.

## Installation

```bash
cargo install brewery
```

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for detailed release notes and version history.

