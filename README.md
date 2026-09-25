# macOS Task Cleaner Core (`macos-task-cleaner-core`)

面向 macOS 的轻量级前台任务清场核心引擎库。提供高精度 GUI 应用扫描、多级白名单防误杀矩阵以及 POSIX 三段式降级清场算法。

---

## 核心架构与模块

* **`app` ([src/app.rs](file:///Users/don/work/git/macos-task-cleaner/src/app.rs))**：
  * 通过 AppKit `NSWorkspace` 原生 API 精准检索当前系统所有处于活跃状态的前台图形应用 (`activationPolicy == Regular`)；
  * 原生排除纯后台守护进程 (Daemon)、Launchd 代理、小组件与状态栏辅助工具。
* **`whitelist` ([src/whitelist.rs](file:///Users/don/work/git/macos-task-cleaner/src/whitelist.rs))**：
  * **L1 系统核心层 (Core OS)**：保护 Finder、Dock、WindowServer、SystemUIServer 等；
  * **L2 会话终端层 (Context Shell)**：自适应保护调用者 PID、父进程 PPID，以及常见终端与 IDE（Ghostty、iTerm2、Terminal、Alacritty、VS Code 等）；
  * **L3 常驻设施层 (Persistent Utilities)**：保护 Raycast、Alfred、Rectangle、输入法（鼠须管、搜狗）与系统监控小组件；
  * **L4 用户配置层 (User Config & CLI)**：支持从 `~/.config/mtc/config.toml` 加载，并提供向配置文件持久化追加规则的接口。
* **`signal` ([src/signal.rs](file:///Users/don/work/git/macos-task-cleaner/src/signal.rs))**：
  * 基于 `libc::kill(pid, 0)` 的无损存活探测；
  * 三段式平滑清场算法：`SIGTERM` 软下线通知 -> 宽限期轮询 -> `SIGKILL` 兜底强退。

---

## 引入依赖

在 `Cargo.toml` 中添加：

```toml
[dependencies]
macos-task-cleaner-core = { git = "https://github.com/DonJone/macos-task-cleaner-core" }
```

---

## 核心接口使用示例

```rust
use std::time::Duration;
use macos_task_cleaner_core::{
    scan_foreground_apps,
    tiered_terminate,
    WhitelistManager,
};

fn main() {
    // 1. 初始化四级白名单引擎
    let (whitelist, config) = WhitelistManager::new(None, &["临时保留应用".to_string()]);

    // 2. 扫描系统前台图形应用
    let all_apps = scan_foreground_apps();

    // 3. 过滤出待清场应用
    let targets: Vec<_> = all_apps
        .into_iter()
        .filter(|app| whitelist.check_protection(app).is_none())
        .collect();

    // 4. 执行平滑分级清场
    let report = tiered_terminate(&targets, Duration::from_millis(400), false);
    println!("清理完成: 成功退出 {} 个应用", report.terminated_sigterm + report.terminated_sigkill);
}
```

---

## 配套工具

* **命令行交互工具 (CLI)**：请参见 [macos-task-cleaner-cli](https://github.com/DonJone/macos-task-cleaner-cli) (`mtc`)

---

## 许可协议

MIT License
