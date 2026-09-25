# macOS Task Cleaner

<p align="left">
  <a href="README.md">English</a> | <a href="README_ZH.md">简体中文</a>
</p>

<p align="left">
  <a href="https://apple.com/macos"><img src="https://img.shields.io/badge/平台-macOS-000000?logo=apple&logoColor=white" alt="平台: macOS" /></a>
  <img src="https://img.shields.io/badge/架构-Apple%20Silicon%20%7C%20AMD64-blue" alt="架构: Apple Silicon | AMD64" />
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/语言-Rust-dea584?logo=rust&logoColor=white" alt="语言: Rust" /></a>
  <img src="https://img.shields.io/badge/Rust-1.75%2B-orange?logo=rust&logoColor=white" alt="Rust: 1.75+" />
  <a href="LICENSE"><img src="https://img.shields.io/badge/开源协议-GNU%20AGPLv3-blue" alt="开源协议: GNU AGPLv3" /></a>
  <a href="COMMERCIAL.md"><img src="https://img.shields.io/badge/商业许可-可授权-orange" alt="商业许可: 可授权" /></a>
</p>

面向 macOS 的高性能、非侵入式前台任务清理与进程管理工具集。由 Rust 编写的高精度核心引擎（`macos-task-cleaner-core`）、终端交互向导命令行工具（`mtc`）以及基于原生 SwiftUI/AppKit 开发的状态栏常驻应用（`TaskCleaner.app`）组成。

---

## 效果展示与界面一览

| 原生菜单栏常驻客户端 (`TaskCleaner.app`) | 终端交互式向导 (`mtc -i`) |
| :---: | :---: |
| <img src="docs/images/gui-menubar.png" width="340" alt="macOS Task Cleaner 菜单栏界面" /> | <img src="docs/images/cli-interactive.png" width="480" alt="macOS Task Cleaner 交互式终端向导" /> |

---

## 核心特性

* **非侵入式 POSIX 分级降级清场**：彻底绕过各类应用层阻塞式保存与确认弹窗，按序列平滑执行优雅退出协议（`SIGTERM` 软下线通知 -> 宽限期毫秒级轮询 -> `SIGKILL` 兜底强退）。
* **四级白名单防御矩阵**：
  * **L1 系统核心层 (Core OS)**：保护 Finder、Dock、WindowServer、SystemUIServer 等系统核心中枢。
  * **L2 会话终端层 (Context Shell)**：自适应保护调用者 PID、父进程 PPID，以及常见开发终端与 IDE（Terminal、Ghostty、iTerm2、Alacritty、VS Code 等）。
  * **L3 常驻设施层 (Persistent Utilities)**：免疫 Raycast、Alfred、Rectangle、输入法（鼠须管、搜狗）与系统监控小组件。
  * **L4 用户配置层 (User Config & CLI)**：支持从 `~/.config/mtc/config.toml` 持久化加载与追加规则。
* **双模态原生交互**：
  * **图形界面 (GUI)**：动态感知前台应用数量，支持窗口高度自适应、单个任务垃圾桶结束、24 种常用语言自动识别与即时切换、扩展项添加白名单及白名单移除。
  * **命令行交互向导 (CLI)**：提供纯键盘操作的向导界面（`mtc -i`），支持预检预览（`--dry-run`）以及为 Raycast/自动化脚本适配的 `--json` 结构化输出。
* **对标 macOS 实用工具原装质感**：严格遵循 Apple Human Interface Guidelines，采用深色监视器屏幕基底、暗调微网格与终端同款微浮雕质感，完美适配 macOS 深浅外观模式。

---

## 项目组件与架构体系

本项目由三个高内聚、低耦合的核心模块构成：

* **[macos-task-cleaner-core](https://github.com/DonJone/macos-task-cleaner-core)**：底层核心引擎库（Rust 开发）。基于 AppKit `NSWorkspace` 原生 API 精确扫描前台图形进程，提供多级白名单评估算法与 POSIX 信号生命周期调度。
* **[macos-task-cleaner-cli](https://github.com/DonJone/macos-task-cleaner-cli) (`mtc`)**：命令行交互式客户端。支持终端向导、快速加白、静默清理与自动化集成。
* **[macos-task-cleaner-gui](https://github.com/DonJone/macos-task-cleaner-gui) (`TaskCleaner.app`)**：状态栏常驻客户端。基于 Swift 与 SwiftUI 架构原生构建，轻量无额外运行时开销。

---

## 快速安装与构建

### 方式一：原生状态栏图形应用 (GUI)

要求 macOS 13.0+ 及 Swift 5.9+ / Xcode 环境：

```bash
git clone https://github.com/DonJone/macos-task-cleaner-gui.git
cd macos-task-cleaner-gui

# 使用内置脚本一键编译并打包 Release 应用
./scripts/build_app.sh

# 安装至应用程序目录
cp -R build/TaskCleaner.app /Applications/
open /Applications/TaskCleaner.app
```

### 方式二：命令行客户端 (CLI)

要求已安装 Rust 工具链（1.75+）：

```bash
git clone https://github.com/DonJone/macos-task-cleaner-cli.git
cd macos-task-cleaner-cli

# 编译 Release 二进制文件
cargo build --release

# 安装主命令到可执行路径
cp target/release/mtc ~/.local/bin/
ln -sf ~/.local/bin/mtc ~/.local/bin/taskcleaner
```

---

## 命令行使用指南 (`mtc`)

```bash
# 启动交互式向导 (日常推荐)
mtc -i

# 预检预览 (仅扫描分析，不杀任何进程)
mtc --dry-run

# 结构化 JSON 格式输出 (适配脚本或 Raycast 插件接入)
mtc --json --dry-run

# 一键追加白名单规则到持久化配置文件
mtc -a "com.google.Chrome"

# 立即执行实质清场
mtc --execute
```

#### 交互式向导指令 (`mtc -i`)

* `w [编号...]`：将指定序号的应用永久加入配置文件白名单（如：`w 1, 2` 或 `w 1 3`）；
* `t [编号...]`：在本轮清场中临时豁免/跳过指定应用；
* `c` / `clean`：确认执行平滑清场；
* `f` / `force`：跳过宽限期直接强制秒杀（直接发送 `SIGKILL`）；
* `p` / `protected`：查看并管理当前受保护的应用清单与各白名单层级；
* `r` / `refresh`：重新扫描系统活跃的前台应用；
* `q` / `quit`：取消并安全退出。

---

## 配置文件规范

配置文件默认位于 `~/.config/mtc/config.toml`（亦向前兼容 `~/.config/taskcleaner/config.toml`）：

```toml
[general]
# 宽限期轮询超时时长 (单位: 毫秒，默认 400ms)
grace_period_ms = 400

# 默认是否以 --dry-run 预检模式运行 (true: 仅扫描分析; false: 直接执行清理)
default_dry_run = false

[whitelist]
# 用户自定义白名单 - 按 Bundle Identifier 豁免 (推荐)
bundle_ids = [
    "com.google.Chrome",
    "com.spotify.client",
]

# 用户自定义白名单 - 按应用显示名称豁免
names = [
    "Telegram",
    "Slack",
]
```

---

## Rust 核心库接口调用示例

可在自有 Rust 项目的 `Cargo.toml` 中直接引用：

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
    println!("清理完成: 成功结束 {} 个前台任务", report.terminated_sigterm + report.terminated_sigkill);
}
```

---

## 许可协议与商业授权

本项目采用双重授权模式（Dual-Licensing Model）：

1. **开源许可证**：遵循 **GNU Affero General Public License v3.0 (AGPLv3)** 协议。个人学习、学术研究与非商业开源项目可免费使用与修改；凡修改或基于本项目构建衍生作品（包括通过网络提供交互服务的 SaaS / 云端调用形态），均须向公众无偿开源全部衍生代码。详见 [LICENSE](LICENSE)。
2. **商业许可协议 (Commercial License)**：面向企业客户、闭源专有产品集成、白标重命名销售或无法遵守 AGPLv3 传染性条款的商业场景，必须事先取得商业授权许可证。详见 [COMMERCIAL.md](COMMERCIAL.md)。
3. **商标与品牌保护**：项目名称、标识图形与应用图标均受版权及商标保护。任何二次分发或分叉 (Fork) 版本必须彻底去除官方品牌元素。详见 [TRADEMARK.md](TRADEMARK.md)。

Copyright (c) 2026 DonJone. 保留所有权利。
