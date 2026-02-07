# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-02-07

### Added
- Upgrade selected leaf with a two-step confirm flow (`Shift+U`)
- Outdated tab in the status panel with installed outdated leaves
- Upgrade-all flow for outdated packages from Status -> Outdated (`Shift+U`, confirm required)
- Outdated-only filter for the leaves list (`o`)
- Brew update recency indicators in Activity (`Brew update` + `Last brew update`)

### Changed
- Status panel now serves as the central Activity/Issues/Outdated workspace
- Action outcomes are surfaced more clearly in Activity with success/error toast lines
- Install/uninstall/upgrade and upgrade-all now refresh status data after completion
- Command feedback in Activity includes clearer failure snippets for package actions

### Refactored
- Renamed health-oriented internals to status-oriented naming (`health` -> `status`)
- Split app module into focused files (`state`, `filters`, `requests`, `reducers`)
- Split status UI rendering into tab-specific builders for maintainability

## [0.2.0] - 2025-02-02

### Added
- Smart scroll debouncing to prevent CPU spikes during rapid navigation
- LRU cache for package details with bounded memory usage (64 entries max)
- Rapid scrolling detection to skip unnecessary detail fetches
- Enhanced release profile optimizations for smaller, faster binaries

### Changed
- **PERFORMANCE**: 3-4x faster health checks through parallel command execution
- **PERFORMANCE**: 50% reduction in CPU usage when idle (tick rate optimized from 250ms to 500ms)
- **PERFORMANCE**: Async startup - UI remains responsive during initial loading
- **PERFORMANCE**: Event-driven redraws instead of constant polling
- Converted `fetch_leaves()` from blocking to async operation
- Details debounce increased from 150ms to 300ms for better scroll performance
- Memory usage is now bounded with LRU eviction policy

### Technical Details
- Parallelized health check commands (`--version`, `info`, `leaves`, `doctor`)
- Implemented conditional rendering with `needs_redraw` flag
- Added `recent_selection_count` tracking for scroll behavior analysis
- Enhanced debounce logic with multiple validation checks
- Upgraded from `HashMap` to `LruCache` for details storage
- Added comprehensive release profile with LTO, symbol stripping, and panic=abort

## [0.1.1] - Previous Release

### Initial Features
- TUI for Homebrew package management
- Package browsing and search functionality
- Health status monitoring
- Size information display
- Theme support (auto, light, dark)
- Package installation/uninstallation
- Bundle dump functionality
