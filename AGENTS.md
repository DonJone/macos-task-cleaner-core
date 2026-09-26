# macOS Task Cleaner Core - Agent Guidelines & Engineering Constraints

This document defines the architectural conventions, engineering rules, and hard constraints for AI coding agents operating on the `macos-task-cleaner-core` engine codebase.

---

## 1. Global Operating Policies

1. **Strict No-Emoji Policy**:
   * Never output Unicode emojis in code, comments, Git commit messages, logs, UI strings, documentation, or responses.
   * Use plain text prefixes for emphasis or status (e.g., `[INFO]`, `[WARN]`, `[SUCCESS]`, `*`, `-`).

2. **Workspace Delivery Principle**:
   * All deliverables, source code modifications, scripts, and documentation must physically persist within the local workspace directory.
   * Never leave deliverables exclusively in hidden cache directories.

3. **Clickable File Links**:
   * All file paths and symbol references in explanations must use the `file://` scheme (e.g. `file:///Users/don/work/git/macos-task-cleaner/src/lib.rs`).

---

## 2. Architecture Overview

`macos-task-cleaner-core` is the foundational Rust engine providing high-precision foreground application discovery and tiered process termination for macOS.

* `src/lib.rs`: Public API facade exporting `scan_foreground_apps`, `tiered_terminate`, and `WhitelistManager`.
* `src/models.rs`: Core data structures (`RunningApp`, `TerminationReport`, `TerminationStrategy`, `ProtectionTier`).
* `src/whitelist.rs`: 4-tier whitelist evaluation matrix and TOML config parser.
* `src/scanner.rs`: Native AppKit `NSWorkspace` foreground application scanner via Objective-C runtime bindings.
* `src/terminate.rs`: POSIX signal management and graceful grace-period polling engine.

---

## 3. Engineering Constraints & Rules

### A. 4-Tier Whitelist Defense Matrix
* **L1 Core OS**: Non-negotiable system processes (Finder, Dock, WindowServer, SystemUIServer, loginwindow). Must always be protected.
* **L2 Context Shell**: Calling process (PID), parent process (PPID), active shell sessions, and development environments (Terminal, Ghostty, iTerm2, Alacritty, VS Code). Must automatically resolve caller lineage to prevent terminating the user's terminal.
* **L3 Persistent Utilities**: Menu bar utilities, window managers, and input methods (Raycast, Alfred, Rectangle, Rime, Sogou).
* **L4 User Configuration**: Persistent rules defined in `~/.config/mtc/config.toml` or CLI overrides (`-k` / `--keep`).

### B. POSIX Termination Safety
* **Sequence**: Always issue `SIGTERM` first, poll process existence across the configured grace period (default 400ms), and escalate to `SIGKILL` only if the process remains unresponsive.
* **Non-Intrusive Execution**: Bypass blocking modal dialogs by dispatching POSIX signals directly rather than invoking UI-level quit actions.

### C. Testing & Verification
* Run `cargo test` after any modifications to `whitelist.rs` or `models.rs`.
* Ensure all existing unit tests in `whitelist::tests` pass with zero regressions.

---

## 4. Licensing & Commercial Policy

* **Dual-Licensing Model**:
  * Open-source under **GNU AGPLv3**. Any derivative library or network service embedding this crate must remain AGPLv3.
  * Commercial closed-source bundling requires a separate commercial license.
* **Documentation**:
  * Maintain `LICENSE`, `COMMERCIAL.md`, and `TRADEMARK.md` at repository root.
