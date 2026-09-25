# macOS Task Cleaner Core (`mtc` / `taskcleaner`)

面向 macOS 的轻量级、工程级前台任务清场工具与执行引擎。采用原生 Rust 开发，提供等同于移动端“一键清理后台”的清爽确定性体验。

---

## 核心特性

* **零弹窗静默下线 (Zero-Dialog Frictionless)**：直接在 POSIX 信号层处理进程终止，绕过应用层事件循环中的各类确认对话框（如未保存草稿、多标签页退出确认等）。
* **三段式分级降级算法 (Tiered Termination)**：
  1. **阶段一**：发送 `SIGTERM` (kill -15)，让应用触发底层的退出句柄，安全回写磁盘并释放资源；
  2. **阶段二**：可配置宽限期轮询（默认 400ms），周期性通过无损信号 0 探测存活状态；
  3. **阶段三**：对超时仍未退出的死锁或无响应顽固进程，发送 `SIGKILL` (kill -9) 兜底强行回收。
* **四级白名单防御体系 (Four-Tier Whitelist)**：
  * **L1 系统核心层 (Core OS)**：强制保护 Finder、Dock、WindowServer、SystemUIServer 等；
  * **L2 会话终端层 (Context Shell)**：自适应保护当前调用者 PID、父进程 PPID，以及常见终端与 IDE（Ghostty、iTerm2、Terminal、Alacritty、VS Code 等）；
  * **L3 常驻设施层 (Persistent Utilities)**：保护 Raycast、Alfred、Rectangle、输入法（鼠须管、搜狗）与系统监控小组件；
  * **L4 用户配置层 (User Config & CLI)**：支持 `~/.config/mtc/config.toml` 持久化配置，以及命令行 `-k / --keep` 临时保留。
* **预检模式与审计 (Dry-Run & Audit)**：默认提供友好的终端预览报表，同时支持 `--json` 输出结构化数据，方便接入 Raycast Script Command 与 macOS 快捷指令。
* **极致性能与零运行时依赖**：原生编译为单一 Mach-O 二进制文件（体积 < 1MB），检索与信号派发全流程耗时仅约 10~25ms，内存占用可忽略不计。

---

## 编译与安装

### 编译构建

要求已安装 Rust 工具链 (1.75+)：

```bash
# 调试构建
cargo build

# 生产级优化构建 (将同时生成 mtc 与 taskcleaner 二进制)
cargo build --release
```

编译生成的可执行文件位于 `target/release/mtc`（与 `target/release/taskcleaner`）。

### 安装到系统路径

```bash
# 复制主命令 mtc 到用户 bin 目录
cp target/release/mtc ~/.local/bin/

# (可选) 同时创建全称 taskcleaner 软链接
ln -sf ~/.local/bin/mtc ~/.local/bin/taskcleaner
```

---

## 使用指南

### 1. 交互式向导模式 (推荐)

启动交互式终端面板，支持直观查看前台应用、按序号一键加白名单、临时豁免及确认清场：

```bash
# 启动交互式向导
mtc -i
# 或
mtc --interactive
```

在交互式会话中：
* `w 1, 2` 或 `1 2`：将序号为 1 和 2 的应用永久写入配置文件白名单；
* `t 1`：本轮清场中临时跳过该应用（不写入文件）；
* `c` 或 `clean`：确认执行平滑清场；
* `f` 或 `force`：直接强制秒杀；
* `p` 或 `protected`：查看当前已被白名单保护的清单；
* `q` 或 `quit`：取消并安全退出。

### 2. 预检模式 (查看待清理应用与白名单命中)

```bash
# 默认预览 (不杀任何进程)
mtc --dry-run

# 临时指定保留特定应用 (支持名称或 Bundle ID)
mtc -k "微信" -k "Google Chrome" --dry-run

# 以结构化 JSON 格式输出 (适合脚本集成)
mtc --json --dry-run
```

### 3. 一键追加白名单 (适用于交付脚本与快速配置)

```bash
# 支持按应用显示名称添加
mtc -a "微信"

# 支持按 Bundle ID 添加 (推荐)
mtc -a "com.spotify.client" -a "com.tencent.xinWeChat"
```

### 4. 实质执行清场

```bash
# 执行标准三段式平滑清场
mtc --execute

# 跳过宽限期直接强退 (秒杀模式)
mtc --force

# 清理后强制回收系统 inactive 内存缓存
mtc --execute --purge
```

### 5. 初始化与管理配置文件

```bash
# 生成默认配置文件模板至 ~/.config/mtc/config.toml
mtc --init-config

# 使用指定的自定义配置文件运行
mtc -c /path/to/custom-config.toml --dry-run
```

---

## 配置文件示例

配置文件优先位于 `~/.config/mtc/config.toml`（亦兼容 `~/.config/taskcleaner/config.toml`）：

```toml
[general]
# 宽限期轮询超时时长 (单位: 毫秒，默认 400ms)
grace_period_ms = 400

# 默认是否以 --dry-run 预检模式运行 (true: 需显式传 -e 才会执行清理; false: 默认执行清理)
default_dry_run = false

[whitelist]
# 用户自定义白名单 - 按 Bundle Identifier 豁免 (推荐)
bundle_ids = [
    # "com.apple.Music",
    # "com.spotify.client",
    # "com.tencent.xinWeChat",
]

# 用户自定义白名单 - 按应用显示名称豁免
names = [
    # "Music",
    # "微信",
    # "Slack",
]
```

---

## 命令行完整参数说明

```text
用法:
  mtc [选项]   (或 taskcleaner [选项])

核心选项:
  -i, --interactive         交互式清场向导 (推荐: 支持序号选择、一键添加白名单与确认清场)
  -n, --dry-run             预检预览模式 (仅扫描并分析白名单过滤，不发送任何终止信号)
  -e, --execute             执行实质清场动作 (执行 SIGTERM -> 轮询 -> SIGKILL 三段式下线)
  -f, --force               强制直接秒杀 (跳过宽限期，直接发送 SIGKILL)
  -a, --add-whitelist <ID>  向永久配置文件追加白名单规则 (支持名称或 Bundle ID，如: -a 微信)
  -k, --keep <NAME/BUNDLE>  命令行临时追加豁免白名单 (仅对当前进程生效，支持多次传入)
  -p, --purge               清场完成后调用 /usr/sbin/purge 强制回收内存缓存
  -c, --config <FILE>       指定自定义 TOML 配置文件路径
      --init-config         在 ~/.config/mtc/config.toml 生成默认配置模板
      --json                以结构化 JSON 格式输出结果 (适配 Raycast / 脚本接入)
  -h, --help                显示帮助说明
  -v, --version             显示当前版本
```

---

## 许可协议

MIT License
