# macOS Task Cleaner

<p align="left">
  <a href="README.md">English</a> | <a href="README_ZH.md">简体中文</a>
</p>

<p align="left">
  <a href="https://apple.com/macos"><img src="https://img.shields.io/badge/Platform-macOS-000000?logo=apple&logoColor=white" alt="Platform: macOS" /></a>
  <img src="https://img.shields.io/badge/Architecture-Apple%20Silicon%20%7C%20AMD64-blue" alt="Architecture: Apple Silicon | AMD64" />
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Language-Rust-dea584?logo=rust&logoColor=white" alt="Language: Rust" /></a>
  <img src="https://img.shields.io/badge/Rust-1.75%2B-orange?logo=rust&logoColor=white" alt="Rust: 1.75+" />
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-GNU%20AGPLv3-blue" alt="License: GNU AGPLv3" /></a>
  <a href="COMMERCIAL.md"><img src="https://img.shields.io/badge/Commercial-License%20Available-orange" alt="Commercial License Available" /></a>
</p>

A high-performance, non-intrusive foreground task cleaner and process manager designed natively for macOS. Powered by a high-precision Rust core engine (`macos-task-cleaner-core`), an interactive command-line wizard (`mtc`), and a sleek native SwiftUI/AppKit menu bar utility (`TaskCleaner.app`).

---

## Overview & Interface Showcase

| Native Menu Bar Utility (`TaskCleaner.app`) | Interactive CLI Wizard (`mtc -i`) |
| :---: | :---: |
| <img src="docs/images/gui-menubar.png" width="340" alt="macOS Task Cleaner Menu Bar GUI" /> | <img src="docs/images/cli-interactive.png" width="480" alt="macOS Task Cleaner Interactive CLI" /> |

---

## Key Features

* **Non-Intrusive POSIX Tiered Termination**: Bypasses blocking modal save/confirm dialogs by issuing an orderly, tiered shutdown sequence (`SIGTERM` soft termination -> polling grace period -> `SIGKILL` fallback).
* **4-Tier Whitelist Defense Matrix**:
  * **L1 Core OS**: Protects essential system processes (Finder, Dock, WindowServer, SystemUIServer).
  * **L2 Context Shell**: Automatically immunizes the caller's PID, parent PPID, active shell sessions, and popular developer tools (Terminal, Ghostty, iTerm2, Alacritty, VS Code).
  * **L3 Persistent Utilities**: Protects system menu bar tools, window managers, and input methods (Raycast, Alfred, Rectangle, Rime, Sogou).
  * **L4 User Configuration**: Supports persistent rules managed via `~/.config/mtc/config.toml`.
* **Dual Native Interfaces**:
  * **Menu Bar Extra (GUI)**: Live running task badge, adaptive window height, individual task termination, 24-language automatic localization, and one-click whitelist toggling.
  * **Terminal Wizard (CLI)**: Keyboard-driven interactive console (`mtc -i`) with quick index-based actions, pre-flight dry runs, and JSON output support.
* **Authentic macOS System Utility Craft**: Follows Apple Human Interface Guidelines with a native monitor screen chassis, subtle technical gridlines, and full light/dark appearance support.

---

## Ecosystem Architecture

The project is structured into three decoupled, complementary components:

* **[macos-task-cleaner-core](https://github.com/DonJone/macos-task-cleaner-core)**: The core engine crate written in Rust. Provides `NSWorkspace` foreground process scanning, whitelist evaluation, and POSIX signal management.
* **[macos-task-cleaner-cli](https://github.com/DonJone/macos-task-cleaner-cli) (`mtc`)**: The command-line client providing interactive wizards, scripting automation, and dry-run diagnostics.
* **[macos-task-cleaner-gui](https://github.com/DonJone/macos-task-cleaner-gui) (`TaskCleaner.app`)**: The native macOS menu bar status item application built with Swift and SwiftUI.

---

## Quick Start & Installation

### Option 1: Native Menu Bar Application (GUI)

Requires macOS 13.0+ and Xcode / Swift 5.9+:

```bash
git clone https://github.com/DonJone/macos-task-cleaner-gui.git
cd macos-task-cleaner-gui

# Build the release bundle
./scripts/build_app.sh

# Install to Applications
cp -R build/TaskCleaner.app /Applications/
open /Applications/TaskCleaner.app
```

### Option 2: Command-Line Interface (CLI)

Requires Rust toolchain (1.75+):

```bash
git clone https://github.com/DonJone/macos-task-cleaner-cli.git
cd macos-task-cleaner-cli

# Compile release binary
cargo build --release

# Install binary to local path
cp target/release/mtc ~/.local/bin/
ln -sf ~/.local/bin/mtc ~/.local/bin/taskcleaner
```

---

## Usage Guide

### Command-Line Usage (`mtc`)

```bash
# Launch interactive wizard (Recommended)
mtc -i

# Pre-flight preview without terminating any processes
mtc --dry-run

# Structured JSON output for scripting and Raycast extensions
mtc --json --dry-run

# Add an application to persistent whitelist
mtc -a "com.google.Chrome"

# Immediate execution of tiered cleanup
mtc --execute
```

#### Interactive Console Shortcuts (`mtc -i`)

* `w [indices]`: Add specified applications permanently to the configuration whitelist (e.g. `w 1, 2`).
* `t [indices]`: Temporarily skip applications for the current cleaning cycle.
* `c` / `clean`: Confirm and execute smooth tiered cleanup.
* `f` / `force`: Immediate termination bypassing grace periods (`SIGKILL`).
* `p` / `protected`: Inspect currently protected applications and whitelist tiers.
* `r` / `refresh`: Rescan active foreground applications.
* `q` / `quit`: Cancel and exit safely.

---

## Configuration

Configuration is stored at `~/.config/mtc/config.toml` (compatible with `~/.config/taskcleaner/config.toml`):

```toml
[general]
# Polling grace period timeout before falling back to SIGKILL (in milliseconds, default: 400ms)
grace_period_ms = 400

# Default execution mode (false: execute cleanup; true: dry-run only)
default_dry_run = false

[whitelist]
# Whitelist by Bundle Identifier (Recommended)
bundle_ids = [
    "com.google.Chrome",
    "com.spotify.client",
]

# Whitelist by Application Display Name
names = [
    "Telegram",
    "Slack",
]
```

---

## Rust Core Library Usage

To integrate the engine into your own Rust project:

```toml
[dependencies]
macos-task-cleaner-core = { git = "https://github.com/DonJone/macos-task-cleaner-core" }
```

```rust
use std::time::Duration;
use macos_task_cleaner_core::{
    scan_foreground_apps,
    tiered_terminate,
    WhitelistManager,
};

fn main() {
    let (whitelist, _config) = WhitelistManager::new(None, &[]);
    let apps = scan_foreground_apps();
    let targets: Vec<_> = apps
        .into_iter()
        .filter(|app| whitelist.check_protection(app).is_none())
        .collect();

    let report = tiered_terminate(&targets, Duration::from_millis(400), false);
    println!("Cleaned {} processes", report.terminated_sigterm + report.terminated_sigkill);
}
```

---

## License & Commercial Terms

This project is dual-licensed:

1. **Open-Source License**: Licensed under the **GNU Affero General Public License v3.0 (AGPLv3)** for individual, academic, and non-commercial open-source usage. Under this license, any derivative work, modification, or network-accessible service utilizing this codebase must release its complete corresponding source code under the AGPLv3. See [LICENSE](LICENSE) for details.
2. **Commercial License**: For enterprise deployment, proprietary closed-source bundling, white-labeling, or integration into commercial utilities where AGPLv3 compliance cannot be met, a separate commercial license is required. See [COMMERCIAL.md](COMMERCIAL.md) for licensing terms and acquisition details.
3. **Trademark Policy**: All product names, logos, and icon assets are protected. Forked distributions must be de-branded. See [TRADEMARK.md](TRADEMARK.md).

Copyright (c) 2026 DonJone. All rights reserved.
