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
- `j`/`k` or `↑`/`↓`: move selection
- `/`: search leaves
- `f`: search packages
- `i`: install package
- `u`: uninstall package
- `Enter`: load details for selected package
- `d`: load deps and reverse deps for selected package
- `r`: refresh leaves list
- `s`: load sizes
- `h`: health check
- `c`: cleanup
- `a`: autoremove
- `b`: bundle dump
- `v`: toggle details/results view
- `Tab`/`Shift+Tab`: cycle focus
- `t`: toggle theme (auto/light/dark)
- `Alt+i`: toggle Nerd/ASCII icons
- `?`: help
- `q` or `Esc`: quit

## Requirements
- Homebrew installed and available as `brew`
- Rust toolchain (edition 2024)

## Font
- Nerd Font is optional; set `BREWERY_ASCII=1` or press `Alt+i` to use ASCII icons.

## Installation

```bash
cargo install brewery
```

