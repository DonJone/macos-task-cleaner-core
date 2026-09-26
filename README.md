# macOS Task Cleaner

<p align="left">
  <a href="README.md">English</a> | <a href="README_ZH.md">简体中文</a>
</p>

<p align="left">
  <a href="https://apple.com/macos"><img src="https://img.shields.io/badge/Platform-macOS%2013%2B-000000?logo=apple&logoColor=white" alt="Platform: macOS 13+" /></a>
  <img src="https://img.shields.io/badge/Architecture-Apple%20Silicon%20%7C%20AMD64-blue" alt="Architecture: Apple Silicon | AMD64" />
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Engine-Rust%201.75%2B-dea584?logo=rust&logoColor=white" alt="Engine: Rust 1.75+" /></a>
  <a href="https://swift.org/"><img src="https://img.shields.io/badge/UI-SwiftUI%20%7C%20AppKit-F05138?logo=swift&logoColor=white" alt="UI: SwiftUI | AppKit" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-GNU%20AGPLv3-blue" alt="License: GNU AGPLv3" /></a>
  <a href="COMMERCIAL.md"><img src="https://img.shields.io/badge/Commercial-License%20Available-orange" alt="Commercial License Available" /></a>
</p>

A high-performance, non-intrusive foreground task cleaner and process manager designed natively for macOS. Powered by a high-precision Rust core engine (`macos-task-cleaner-core`), an interactive command-line wizard (`mtc`), and a sleek native SwiftUI/AppKit menu bar utility (`TaskCleaner.app`).

---

## Visual Interface Showcase

### Dual-Modal Interactive Experience

| Native Menu Bar Popover (`TaskCleaner.app`) | Interactive Terminal Wizard (`mtc -i`) |
| :---: | :---: |
| <img src="docs/images/gui-main-en.png" width="360" alt="macOS Task Cleaner Menu Bar Popover" /> | <img src="docs/images/cli-interactive-en.png" width="520" alt="macOS Task Cleaner Interactive Console Wizard" /> |

### Automated Batch Cleanup & Diagnostics

| Tiered Process Termination Report (`mtc --execute`) |
| :---: |
| <img src="docs/images/cli-exec-en.png" width="890" alt="macOS Task Cleaner Batch Execution Report" /> |

---

## Architectural Highlights

* **Non-Intrusive POSIX Tiered Termination**:
  Bypasses application-level blocking modal save/confirm dialogs by orchestrating an orderly tiered shutdown sequence (`SIGTERM` soft termination -> polling grace period -> `SIGKILL` fallback).
* **Native AppKit Finder Voluntary Quit**:
  Unlike traditional process managers that kill Finder with POSIX signals (causing `launchd` to interpret the exit as a crash and immediately respawn it), Task Cleaner issues native AppKit `NSRunningApplication.terminate()` to ensure an orderly voluntary exit without respawning.
* **4-Tier Whitelist Defense Matrix**:
  * **L1 Core OS**: Protects essential system daemons (`Dock`, `WindowServer`, `SystemUIServer`, `ControlCenter`, `NotificationCenter`, `loginwindow`) and default-protected `Finder`.
  * **L2 Context Shell**: Automatically resolves caller lineage (`PID` and `PPID`), protecting active shell sessions and developer environments (`Terminal`, `Ghostty`, `iTerm2`, `Alacritty`, `VS Code`).
  * **L3 Persistent Utilities**: Protects background menu bar utilities, window managers, and input methods (`Raycast`, `Alfred`, `Rectangle`, `Rime`, `Sogou`).
  * **L4 User Configuration**: Supports persistent rules managed via `~/.config/mtc/config.toml` (bundle identifiers, display names) and CLI flags.
* **Authentic macOS System Utility Craft**:
  Follows Apple Human Interface Guidelines with dark chassis styling, technical micro-grid accents, dynamic window height adaptation, and 24-language native localization.

---

## Ecosystem Architecture

The project is structured into three decoupled, complementary components:

* **[macos-task-cleaner-core](https://github.com/macos-task-cleaner/macos-task-cleaner-core)**: The foundational engine crate written in Rust. Provides native `NSWorkspace` foreground process discovery, multi-tier whitelist evaluation matrix, and POSIX signal management.
* **[macos-task-cleaner-cli](https://github.com/macos-task-cleaner/macos-task-cleaner-cli) (`mtc`)**: The command-line client providing interactive wizards (`mtc -i`), batch execution, pre-flight dry runs (`--dry-run`), and JSON diagnostics (`--json`).
* **[macos-task-cleaner-gui](https://github.com/macos-task-cleaner/macos-task-cleaner-gui) (`TaskCleaner.app`)**: The native macOS menu bar status item application built with Swift and SwiftUI, featuring real-time badge counts, individual app controls, and whitelist editing.

---

## Quick Start & Installation

### Option 1: Pre-Built Releases (DMG & Archive)

Download pre-compiled binaries from GitHub Releases:

* **GUI App**: Download `TaskCleaner-macOS-arm64.dmg` or `TaskCleaner-macOS-universal.dmg` from [Task Cleaner GUI Releases](https://github.com/macos-task-cleaner/macos-task-cleaner-gui/releases/latest). Open the disk image and drag `Task Cleaner.app` to your `/Applications` directory.
* **CLI Binary**: Download `mtc-macos-arm64.tar.gz` or `mtc-macos-universal.tar.gz` from [Task Cleaner CLI Releases](https://github.com/macos-task-cleaner/macos-task-cleaner-cli/releases/latest). Extract and place `mtc` into `/usr/local/bin/` or `~/.local/bin/`.

### Option 2: Build from Source

#### Building the GUI App (`TaskCleaner.app`)

Requires macOS 13.0+ and Xcode / Swift 5.9+:

```bash
git clone https://github.com/macos-task-cleaner/macos-task-cleaner-gui.git
cd macos-task-cleaner-gui

# Compile and package Release bundle (supports: arm64 | x86_64 | universal)
./scripts/build_app.sh

# Install to Applications
cp -R build/TaskCleaner.app /Applications/
open /Applications/TaskCleaner.app
```

#### Building the CLI (`mtc`)

Requires Rust toolchain (1.75+):

```bash
git clone https://github.com/macos-task-cleaner/macos-task-cleaner-cli.git
cd macos-task-cleaner-cli

# Compile release binary
cargo build --release

# Install binary to local path
cp target/release/mtc ~/.local/bin/
ln -sf ~/.local/bin/mtc ~/.local/bin/taskcleaner
```

---

## Usage Guide

### 1. Interactive Terminal Wizard (`mtc -i`)

Launch the keyboard-driven interactive console to inspect running foreground applications, check whitelist protection levels, and selectively terminate or whitelist tasks:

```bash
mtc -i
```

#### Interactive Commands Reference

| Command | Syntax | Description | Example |
| :--- | :--- | :--- | :--- |
| `w` | `w <indices>` | Permanently add apps to persistent user whitelist | `w 2, 4` |
| `t` | `t <indices>` | Temporarily exempt apps for the current cleaning cycle | `t 1` |
| `c` / `clean` | `c` | Execute smooth tiered cleanup (`SIGTERM` -> grace period -> `SIGKILL`) | `c` |
| `f` / `force` | `f` | Immediately force kill non-whitelisted apps (`SIGKILL`) | `f` |
| `p` / `protected` | `p` | Inspect currently protected apps and whitelist tiers | `p` |
| `r` / `refresh` | `r` | Rescan running foreground applications from system | `r` |
| `q` / `quit` | `q` | Exit the wizard without making changes | `q` |

### 2. Command-Line Batch Operations

```bash
# Pre-flight preview without terminating any processes
mtc --dry-run

# Execute tiered cleanup with confirmation prompt
mtc

# Execute immediate non-interactive tiered cleanup (suitable for cron/scripts)
mtc --execute

# Execute forced cleanup bypassing grace periods
mtc --force

# Structured JSON output for scripting, Shortcuts, or Raycast extensions
mtc --json --dry-run

# Add an application to persistent user whitelist
mtc -a "com.google.Chrome"

# Remove an application from persistent user whitelist
mtc -r "com.google.Chrome"

# List all active whitelist rules
mtc --list-whitelist
```

### 3. Native Menu Bar Application (`TaskCleaner.app`)

* **Menu Bar Status Item**: Displays a live badge counter reflecting the number of active foreground tasks.
* **Three-Part Summary**:
  * **Pending Cleanup**: Shows count of foreground apps scheduled for termination.
  * **Protected / Whitelisted**: Displays count of immune applications across L1-L4 tiers.
  * **Total Active Apps**: Displays overall active foreground process count.
* **Granular Process Controls**:
  * Click the **Trash** icon next to any process to terminate it individually.
  * Click the **Lock / Shield** icon to toggle persistent whitelist protection.
* **One-Click Batch Cleanup**: Click the **Clean All** button to cleanly terminate all unwhitelisted foreground tasks at once.
* **Finder Voluntary Quit**: Gracefully quits Finder when unwhitelisted without triggering `launchd` auto-respawn loops.
* **Native Localization**: Automatically adapts UI text to system locale across 24 supported languages.

---

## Configuration Specification

Configuration is stored at `~/.config/mtc/config.toml` (compatible with `~/.config/taskcleaner/config.toml`):

```toml
[general]
# Polling grace period timeout before falling back to SIGKILL (in milliseconds, default: 400ms)
grace_period_ms = 400

# Default execution mode (false: execute cleanup; true: dry-run only)
default_dry_run = false

[whitelist]
# Whitelist by Bundle Identifier (Recommended for precision)
bundle_ids = [
    "com.google.Chrome",
    "com.spotify.client",
    "com.tencent.xinWeChat",
]

# Whitelist by Application Display Name
names = [
    "Telegram",
    "Slack",
    "MacVim",
]
```

---

## Rust Core Library Integration

To integrate the `macos-task-cleaner-core` engine directly into your own Rust project:

```toml
[dependencies]
macos-task-cleaner-core = { git = "https://github.com/macos-task-cleaner/macos-task-cleaner-core" }
```

```rust
use std::time::Duration;
use macos_task_cleaner_core::{
    scan_foreground_apps,
    tiered_terminate,
    WhitelistManager,
};

fn main() {
    // 1. Initialize whitelist manager with optional config file and CLI overrides
    let (whitelist, _config) = WhitelistManager::new(None, &[]);

    // 2. Scan active foreground applications via AppKit NSWorkspace
    let apps = scan_foreground_apps();

    // 3. Filter targets excluding protected tiers (L1 - L4)
    let targets: Vec<_> = apps
        .into_iter()
        .filter(|app| whitelist.check_protection(app).is_none())
        .collect();

    // 4. Dispatch tiered termination (SIGTERM -> 400ms grace period -> SIGKILL)
    let report = tiered_terminate(&targets, Duration::from_millis(400), false);
    println!("Cleaned {} processes", report.terminated_sigterm + report.terminated_sigkill);
}
```

---

## License & Commercial Terms

This project is dual-licensed:

1. **Open-Source License**: Licensed under the **GNU Affero General Public License v3.0 (AGPLv3)** for individual, academic, and non-commercial open-source usage. Any derivative work, modification, or network-accessible service utilizing this codebase must release its complete corresponding source code under the AGPLv3. See [LICENSE](LICENSE) for details.
2. **Commercial License**: For enterprise deployment, proprietary closed-source bundling, white-labeling, or integration into commercial utilities where AGPLv3 compliance cannot be met, a separate commercial license is required. See [COMMERCIAL.md](COMMERCIAL.md) for licensing terms and acquisition details.
3. **Trademark Policy**: All product names, logos, and icon assets are protected. Forked distributions must be de-branded. See [TRADEMARK.md](TRADEMARK.md).

Copyright (c) 2026 DonJone. All rights reserved.
