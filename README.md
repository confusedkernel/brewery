# Brewery

Brewery is a Rust TUI for Homebrew. WIP.

## Features
- Leaves browser with instant search filter
- Selection with details panel (description, homepage, installed versions)
- Optional deps and reverse deps loading
- Light/dark theme with auto-detection and manual toggle

## Controls
- `j`/`k` or `↑`/`↓`: move selection
- `/`: focus search
- `Enter`: load details for selected package
- `d`: load deps and reverse deps for selected package
- `r`: refresh leaves list
- `t`: toggle theme (auto/light/dark)
- `q` or `Esc`: quit

## Requirements
- Homebrew installed and available as `brew`
- Rust toolchain (edition 2024)

## Run (current implementation)
```bash
cargo run
```
