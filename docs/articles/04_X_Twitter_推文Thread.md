# X (Twitter) 推文发布串 (Thread) 草稿

## 中文发布串 (Chinese Thread)

### [Tweet 1/4 - 主推文]
桌面堆满卡顿软件？退出时总被“是否保存”弹窗拦截？
我为 macOS 打造了一款轻量开源的清理利器：Task Cleaner (v1.0.0 正式发布)！
原生 SwiftUI 菜单栏面板 + Rust 核心引擎，一键彻底平滑清场，告别阻塞卡死。
开源地址与下载：
https://github.com/macos-task-cleaner/macos-task-cleaner-gui
[配图: images/gui-main-zh.png]

---

### [Tweet 2/4 - 核心技术]
为什么传统清理脚本容易翻车？
1. 传统 killall Finder 会被 launchd 视为崩溃并立即强制自启复活，Task Cleaner 采用独家 AppKit 退出协议确保平稳离线；
2. 优雅下发 SIGTERM -> 400ms 宽限期轮询 -> 超时 SIGKILL 兜底，彻底终结各类流氓未保存模态弹窗！
[配图: images/cli-exec-zh.png]

---

### [Tweet 3/4 - 白名单与双模态]
最懂开发者的 4 级白名单防御：
- 自动保护调用者 PID 与父进程 PPID，绝不误退终端；
- 预置免疫 Terminal, Ghostty, iTerm2, Alacritty, VS Code；
- 智能识别 Raycast, Alfred 与输入法；
- 支持顶部状态栏点击，更提供专为键盘党设计的终端交互向导 (mtc -i)！
[配图: images/cli-interactive-zh.png]

---

### [Tweet 4/4 - 下载与参与]
Task Cleaner 现已全面开源 (GNU AGPLv3)：
- 原生 SwiftUI 构建，零 Electron 开销，支持 24 种语言自适应；
- 支持 Apple Silicon (M1/M2/M3/M4) 与 Intel 架构；
- 前往 GitHub Releases 即可直接下载 DMG 拖拽安装体验：
https://github.com/macos-task-cleaner/macos-task-cleaner-gui/releases/latest
欢迎试用与 Star 关注！

---

---

## 英文发布串 (English Thread)

### [Tweet 1/4 - Main Tweet]
Tired of closing dozen Mac apps only to be trapped by endless "Do you want to save changes?" modal dialogs?
Meet Task Cleaner for macOS (v1.0.0 is officially live)!
Native SwiftUI menu bar popover + high-precision Rust engine for smooth, zero-dialog foreground cleanup.
Open source on GitHub:
https://github.com/macos-task-cleaner/macos-task-cleaner-gui
[Attach: images/gui-main-en.png]

---

### [Tweet 2/4 - Technical Highlights]
Under the hood:
* Tiered Process Termination: Orderly SIGTERM -> 400ms polling grace period -> SIGKILL fallback. Unresponsive modal prompts will never lock your screen again.
* Finder Voluntary Quit: POSIX signals cause launchd to respawn Finder immediately. Task Cleaner uses native AppKit terminate() to ensure clean voluntary quits.
[Attach: images/cli-exec-en.png]

---

### [Tweet 3/4 - Developer Protection]
Bulletproof 4-Tier Whitelist Matrix:
- Automatically resolves caller lineage (PID/PPID), immunizing active shells;
- Built-in immunity for Terminal, Ghostty, iTerm2, Alacritty, and VS Code;
- Shields background utilities (Raycast, Alfred, input methods);
- Includes an interactive terminal console wizard (`mtc -i`) with instant index actions.
[Attach: images/cli-interactive-en.png]

---

### [Tweet 4/4 - Get Started]
Task Cleaner is 100% open source under GNU AGPLv3.
- Native Apple HIG compliance, dark chassis, and 24-language localization;
- Universal Mach-O support (Apple Silicon M1-M4 & Intel);
- Download pre-built DMG installers directly from GitHub Releases:
https://github.com/macos-task-cleaner/macos-task-cleaner-gui/releases/latest
Give it a spin and star the repo if you like it!
