use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::app::AppTarget;

/// 四级白名单分级定义
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WhitelistTier {
    /// L1: 系统核心层 (Core OS) - 保护系统关键图形进程
    L1CoreOs,
    /// L2: 会话终端层 (Context Shell) - 保护当前命令执行宿主与常用终端/IDE
    L2ContextShell,
    /// L3: 基础设施与效率工具 (Persistent Utilities) - 保护输入法、窗口管理等
    L3PersistentUtilities,
    /// L4: 用户自定义配置 (User Config) - 从 ~/.config/taskcleaner/config.toml 读取
    L4UserConfig,
    /// L4: 命令行临时追加 (CLI Override) - 通过 -k / --keep 临时指定
    L4CliOverride,
}

impl WhitelistTier {
    /// 默认中文标签 (保留向后兼容)
    pub fn label(&self) -> &'static str {
        self.localized_label(crate::i18n::Language::ZhHans)
    }
}

/// 白名单命中结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhitelistMatch {
    pub tier: WhitelistTier,
    pub tier_label: String,
    pub matched_rule: String,
    #[serde(default = "default_tier_id")]
    pub tier_id: String,
}

fn default_tier_id() -> String {
    "unknown".to_string()
}

/// 用户配置文件结构定义
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskCleanerConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub whitelist: WhitelistSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_grace_period")]
    pub grace_period_ms: u64,
    #[serde(default = "default_dry_run")]
    pub default_dry_run: bool,
    #[serde(default)]
    pub language: Option<String>,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            grace_period_ms: default_grace_period(),
            default_dry_run: default_dry_run(),
            language: None,
        }
    }
}

fn default_grace_period() -> u64 {
    400
}

fn default_dry_run() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WhitelistSection {
    #[serde(default)]
    pub bundle_ids: Vec<String>,
    #[serde(default)]
    pub names: Vec<String>,
    #[serde(default)]
    pub disabled_rules: Vec<String>,
}

/// 白名单矩阵管理器
pub struct WhitelistManager {
    // L1: 系统核心层
    l1_bundle_ids: HashSet<String>,
    l1_names: HashSet<String>,

    // L2: 会话终端与编辑器
    l2_pids: HashSet<i32>,
    l2_bundle_ids: HashSet<String>,
    l2_names: HashSet<String>,

    // L3: 基础设施与输入法
    l3_bundle_ids: HashSet<String>,
    l3_names: HashSet<String>,

    // L4: 用户配置文件
    l4_user_bundle_ids: HashSet<String>,
    l4_user_names: HashSet<String>,

    // L4: CLI 命令行临时指定
    cli_keep_rules: HashSet<String>,

    // 禁用规则集合 (涵盖对 Core 预设的禁用与移除)
    disabled_rules: HashSet<String>,

    pub loaded_config_path: Option<PathBuf>,
}

impl WhitelistManager {
    pub fn new(
        custom_config_path: Option<&Path>,
        cli_keeps: &[String],
    ) -> (Self, TaskCleanerConfig) {
        let (config, config_path) = Self::load_config(custom_config_path);

        // 初始化 L1: 系统核心层
        let mut l1_bundle_ids = HashSet::new();
        l1_bundle_ids.insert("com.apple.finder".to_string());
        l1_bundle_ids.insert("com.apple.dock".to_string());
        l1_bundle_ids.insert("com.apple.WindowManager".to_string());
        l1_bundle_ids.insert("com.apple.systemuiserver".to_string());
        l1_bundle_ids.insert("com.apple.controlcenter".to_string());
        l1_bundle_ids.insert("com.apple.notificationcenterui".to_string());
        l1_bundle_ids.insert("com.apple.loginwindow".to_string());

        let mut l1_names = HashSet::new();
        l1_names.insert("Finder".to_string());
        l1_names.insert("访达".to_string());
        l1_names.insert("Dock".to_string());
        l1_names.insert("WindowServer".to_string());

        // 初始化 L2: 会话终端与 IDE 保护 (包含全部祖先进程会话链路)
        let l2_pids = crate::signal::get_caller_lineage();

        let mut l2_bundle_ids = HashSet::new();
        l2_bundle_ids.insert("com.apple.Terminal".to_string());
        l2_bundle_ids.insert("com.googlecode.iterm2".to_string());
        l2_bundle_ids.insert("com.mitchellh.ghostty".to_string());
        l2_bundle_ids.insert("io.alacritty".to_string());
        l2_bundle_ids.insert("net.kovidgoyal.kitty".to_string());
        l2_bundle_ids.insert("org.wezfurlong.wezterm".to_string());
        l2_bundle_ids.insert("com.github.wez.wezterm".to_string());
        l2_bundle_ids.insert("dev.warp.Warp-Stable".to_string());
        l2_bundle_ids.insert("com.microsoft.VSCode".to_string());
        l2_bundle_ids.insert("com.sublimetext.4".to_string());
        l2_bundle_ids.insert("com.jetbrains.intellij".to_string());

        let mut l2_names = HashSet::new();
        l2_names.insert("Terminal".to_string());
        l2_names.insert("终端".to_string());
        l2_names.insert("iTerm2".to_string());
        l2_names.insert("Ghostty".to_string());
        l2_names.insert("Alacritty".to_string());
        l2_names.insert("kitty".to_string());
        l2_names.insert("wezterm-gui".to_string());
        l2_names.insert("Warp".to_string());
        l2_names.insert("Code".to_string());
        l2_names.insert("Visual Studio Code".to_string());

        // 初始化 L3: 基础设施与效率工具
        let mut l3_bundle_ids = HashSet::new();
        l3_bundle_ids.insert("com.raycast.macos".to_string());
        l3_bundle_ids.insert("com.runningwithcrayons.Alfred".to_string());
        l3_bundle_ids.insert("com.knollsoft.Rectangle".to_string());
        l3_bundle_ids.insert("com.lwouis.alt-tab-macos".to_string());
        l3_bundle_ids.insert("com.sogou.inputmethod.sogou".to_string());
        l3_bundle_ids.insert("im.rime.inputmethod.Squirrel".to_string());
        l3_bundle_ids.insert("com.apple.inputmethod.SCIM".to_string());
        l3_bundle_ids.insert("com.exelban.Stats".to_string());
        l3_bundle_ids.insert("com.surteesstudios.Bartender".to_string());

        let mut l3_names = HashSet::new();
        l3_names.insert("Raycast".to_string());
        l3_names.insert("Alfred".to_string());
        l3_names.insert("Rectangle".to_string());
        l3_names.insert("AltTab".to_string());
        l3_names.insert("Stats".to_string());
        l3_names.insert("Bartender".to_string());

        // 初始化 L4: 用户配置白名单
        let mut l4_user_bundle_ids = HashSet::new();
        let mut l4_user_names = HashSet::new();
        for bid in &config.whitelist.bundle_ids {
            l4_user_bundle_ids.insert(bid.trim().to_string());
        }
        for name in &config.whitelist.names {
            l4_user_names.insert(name.trim().to_string());
        }

        // 初始化 L4: 命令行临时参数 (-k / --keep)
        let mut cli_keep_rules = HashSet::new();
        for rule in cli_keeps {
            cli_keep_rules.insert(rule.trim().to_string());
        }

        // 初始化禁用规则集合 (用户显式移除的规则，含 Core 预设)
        let mut disabled_rules = HashSet::new();
        for rule in &config.whitelist.disabled_rules {
            disabled_rules.insert(rule.trim().to_string());
        }

        let manager = Self {
            l1_bundle_ids,
            l1_names,
            l2_pids,
            l2_bundle_ids,
            l2_names,
            l3_bundle_ids,
            l3_names,
            l4_user_bundle_ids,
            l4_user_names,
            cli_keep_rules,
            disabled_rules,
            loaded_config_path: config_path,
        };

        (manager, config)
    }

    /// 判定目标是否被四级白名单拦截并给出具体理由
    /// 判定目标是否被四级白名单拦截并给出具体理由 (默认中文，保留向后兼容)
    pub fn check_protection(&self, app: &AppTarget) -> Option<WhitelistMatch> {
        self.check_protection_with_lang(app, crate::i18n::Language::ZhHans)
    }

    /// 判定目标是否被四级白名单拦截并给出指定语言的本地化结果与机器标识符
    pub fn check_protection_with_lang(
        &self,
        app: &AppTarget,
        lang: crate::i18n::Language,
    ) -> Option<WhitelistMatch> {
        let is_zh = matches!(lang, crate::i18n::Language::ZhHans | crate::i18n::Language::ZhHant);

        // 1. 检查 L2 PID 自身与祖先会话链路保护 (不可协商，避免误伤调用者会话及其宿主环境)
        if self.l2_pids.contains(&app.pid) {
            let tier = WhitelistTier::L2ContextShell;
            return Some(WhitelistMatch {
                tier,
                tier_label: tier.localized_label(lang).to_string(),
                tier_id: tier.identifier().to_string(),
                matched_rule: if is_zh {
                    format!("进程 PID 处于当前调用者会话链路 ({})", app.pid)
                } else {
                    format!("PID {} in caller session lineage", app.pid)
                },
            });
        }

        // 2. 检查禁用名单 (支持用户移除预设的白名单规则，如终端、输入法或访达)
        // 底层关键守护进程（Dock, WindowServer 等）禁止被禁用
        if !Self::is_critical_system_daemon(&app.bundle_id) && !Self::is_critical_system_daemon(&app.name) {
            if self.disabled_rules.contains(&app.bundle_id)
                || self.disabled_rules.contains(&app.name)
                || self.disabled_rules.iter().any(|r| {
                    app.name.eq_ignore_ascii_case(r) || app.bundle_id.eq_ignore_ascii_case(r)
                })
            {
                return None;
            }
        }

        // 3. 检查 L1 系统核心层
        if self.l1_bundle_ids.contains(&app.bundle_id) {
            let tier = WhitelistTier::L1CoreOs;
            return Some(WhitelistMatch {
                tier,
                tier_label: tier.localized_label(lang).to_string(),
                tier_id: tier.identifier().to_string(),
                matched_rule: if is_zh {
                    format!("系统核心包名: {}", app.bundle_id)
                } else {
                    format!("Core OS bundle ID: {}", app.bundle_id)
                },
            });
        }
        if self.l1_names.contains(&app.name) {
            let tier = WhitelistTier::L1CoreOs;
            return Some(WhitelistMatch {
                tier,
                tier_label: tier.localized_label(lang).to_string(),
                tier_id: tier.identifier().to_string(),
                matched_rule: if is_zh {
                    format!("系统核心应用名: {}", app.name)
                } else {
                    format!("Core OS app name: {}", app.name)
                },
            });
        }

        // 4. 检查 L2 会话终端与编辑器
        if self.l2_bundle_ids.contains(&app.bundle_id) {
            let tier = WhitelistTier::L2ContextShell;
            return Some(WhitelistMatch {
                tier,
                tier_label: tier.localized_label(lang).to_string(),
                tier_id: tier.identifier().to_string(),
                matched_rule: if is_zh {
                    format!("终端/IDE 包名: {}", app.bundle_id)
                } else {
                    format!("Terminal/IDE bundle ID: {}", app.bundle_id)
                },
            });
        }
        if self.l2_names.contains(&app.name) {
            let tier = WhitelistTier::L2ContextShell;
            return Some(WhitelistMatch {
                tier,
                tier_label: tier.localized_label(lang).to_string(),
                tier_id: tier.identifier().to_string(),
                matched_rule: if is_zh {
                    format!("终端/IDE 应用名: {}", app.name)
                } else {
                    format!("Terminal/IDE app name: {}", app.name)
                },
            });
        }

        // 5. 检查 L3 基础设施与效率工具
        if self.l3_bundle_ids.contains(&app.bundle_id) {
            let tier = WhitelistTier::L3PersistentUtilities;
            return Some(WhitelistMatch {
                tier,
                tier_label: tier.localized_label(lang).to_string(),
                tier_id: tier.identifier().to_string(),
                matched_rule: if is_zh {
                    format!("常驻基础设施包名: {}", app.bundle_id)
                } else {
                    format!("Persistent utility bundle ID: {}", app.bundle_id)
                },
            });
        }
        if self.l3_names.contains(&app.name) {
            let tier = WhitelistTier::L3PersistentUtilities;
            return Some(WhitelistMatch {
                tier,
                tier_label: tier.localized_label(lang).to_string(),
                tier_id: tier.identifier().to_string(),
                matched_rule: if is_zh {
                    format!("常驻基础设施应用名: {}", app.name)
                } else {
                    format!("Persistent utility app name: {}", app.name)
                },
            });
        }

        // 6. 检查 L4 命令行临时保留 (-k / --keep)
        if self.cli_keep_rules.contains(&app.bundle_id)
            || self.cli_keep_rules.contains(&app.name)
            || self.cli_keep_rules.iter().any(|r| {
                app.name.eq_ignore_ascii_case(r) || app.bundle_id.eq_ignore_ascii_case(r)
            })
        {
            let tier = WhitelistTier::L4CliOverride;
            return Some(WhitelistMatch {
                tier,
                tier_label: tier.localized_label(lang).to_string(),
                tier_id: tier.identifier().to_string(),
                matched_rule: if is_zh {
                    format!("命令行 -k 参数命中: {} / {}", app.name, app.bundle_id)
                } else {
                    format!("CLI -k keep rule matched: {} / {}", app.name, app.bundle_id)
                },
            });
        }

        // 7. 检查 L4 用户配置文件白名单
        if self.l4_user_bundle_ids.contains(&app.bundle_id) {
            let tier = WhitelistTier::L4UserConfig;
            return Some(WhitelistMatch {
                tier,
                tier_label: tier.localized_label(lang).to_string(),
                tier_id: tier.identifier().to_string(),
                matched_rule: if is_zh {
                    format!("用户配置包名: {}", app.bundle_id)
                } else {
                    format!("User config bundle ID: {}", app.bundle_id)
                },
            });
        }
        if self.l4_user_names.contains(&app.name)
            || self
                .l4_user_names
                .iter()
                .any(|n| n.eq_ignore_ascii_case(&app.name))
        {
            let tier = WhitelistTier::L4UserConfig;
            return Some(WhitelistMatch {
                tier,
                tier_label: tier.localized_label(lang).to_string(),
                tier_id: tier.identifier().to_string(),
                matched_rule: if is_zh {
                    format!("用户配置应用名: {}", app.name)
                } else {
                    format!("User config app name: {}", app.name)
                },
            });
        }

        None
    }

    /// 获取默认配置文件路径: 优先 ~/.config/mtc/config.toml，兼容回退 ~/.config/taskcleaner/config.toml
    pub fn default_config_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        let mtc_path = PathBuf::from(&home).join(".config/mtc/config.toml");
        if mtc_path.exists() {
            return Some(mtc_path);
        }
        let legacy_path = PathBuf::from(&home).join(".config/taskcleaner/config.toml");
        if legacy_path.exists() {
            return Some(legacy_path);
        }
        Some(mtc_path)
    }

    /// 判定指定标识符是否属于底层硬核系统守护进程（如 WindowServer, loginwindow, Dock 等）
    /// 这类进程直接维系系统会话与窗口合成器，绝对不可被终止或移出保护名单。
    /// （注意：Finder 属于常规 GUI 文件管理器，支持原生 AppKit 安全退出，不在此禁制内）
    pub fn is_critical_system_daemon(identifier: &str) -> bool {
        let trimmed = identifier.trim();
        let daemon_bundles = [
            "com.apple.dock",
            "com.apple.WindowManager",
            "com.apple.systemuiserver",
            "com.apple.controlcenter",
            "com.apple.notificationcenterui",
            "com.apple.loginwindow",
        ];
        let daemon_names = [
            "Dock",
            "WindowServer",
            "SystemUIServer",
            "ControlCenter",
            "NotificationCenter",
            "loginwindow",
        ];
        daemon_bundles.iter().any(|b| b.eq_ignore_ascii_case(trimmed))
            || daemon_names.iter().any(|n| n.eq_ignore_ascii_case(trimmed))
    }

    /// 判定指定标识符（Bundle ID 或应用名）是否属于 L1 系统核心默认保护范围（包含 Finder、Dock 等）
    pub fn is_l1_core_os(identifier: &str) -> bool {
        let trimmed = identifier.trim();
        let l1_bundles = [
            "com.apple.finder",
            "com.apple.dock",
            "com.apple.WindowManager",
            "com.apple.systemuiserver",
            "com.apple.controlcenter",
            "com.apple.notificationcenterui",
            "com.apple.loginwindow",
        ];
        let l1_names = [
            "Finder",
            "访达",
            "Dock",
            "WindowServer",
            "SystemUIServer",
            "ControlCenter",
            "NotificationCenter",
            "loginwindow",
        ];
        l1_bundles.iter().any(|b| b.eq_ignore_ascii_case(trimmed))
            || l1_names.iter().any(|n| n.eq_ignore_ascii_case(trimmed))
    }

    /// 从文件加载配置，若文件不存在则返回默认空配置
    pub fn load_config(custom_path: Option<&Path>) -> (TaskCleanerConfig, Option<PathBuf>) {
        let path = custom_path
            .map(|p| p.to_path_buf())
            .or_else(Self::default_config_path);

        if let Some(ref p) = path {
            if p.exists() {
                if let Ok(content) = fs::read_to_string(p) {
                    if let Ok(mut config) = toml::from_str::<TaskCleanerConfig>(&content) {
                        // 强制过滤任何试图禁用底层关键守护进程的规则（允许禁用 Finder）
                        config.whitelist.disabled_rules.retain(|r| !Self::is_critical_system_daemon(r));
                        return (config, Some(p.clone()));
                    }
                }
            }
        }

        (TaskCleanerConfig::default(), None)
    }

    /// 创建并写入默认配置文件模板
    pub fn generate_default_config_file(target_path: Option<&Path>) -> io::Result<PathBuf> {
        let path = target_path
            .map(|p| p.to_path_buf())
            .or_else(Self::default_config_path)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "无法定位用户 HOME 目录"))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let template = r#"# macOS Task Cleaner (mtc) 配置文件
# 文件位置: ~/.config/mtc/config.toml (或 ~/.config/taskcleaner/config.toml)

[general]
# 宽限期轮询超时时长 (单位: 毫秒，默认 400ms)
grace_period_ms = 400

# 默认是否以 --dry-run 预检模式运行 (true: 默认不杀进程需显式加 -e; false: 默认执行清理)
default_dry_run = false

[whitelist]
# 用户自定义白名单 - 按 Bundle Identifier 豁免 (推荐，跨系统语言版本稳定)
bundle_ids = [
    # "com.apple.Music",        # 系统自带音乐播放器
    # "com.spotify.client",      # Spotify 客户端
    # "com.netease.163music",    # 网易云音乐
    # "com.tencent.xinWeChat",   # 微信 (若习惯保持常驻后台)
]

# 用户自定义白名单 - 按应用显示名称豁免
names = [
    # "Music",
    # "音乐",
    # "Spotify",
    # "Slack",
]
"#;

        fs::write(&path, template)?;
        Ok(path)
    }

    /// 持久化追加白名单条目到用户配置文件
    pub fn append_to_user_config(
        custom_path: Option<&Path>,
        bundle_ids: &[String],
        names: &[String],
    ) -> io::Result<(PathBuf, usize)> {
        let path = custom_path
            .map(|p| p.to_path_buf())
            .or_else(Self::default_config_path)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "无法定位用户 HOME 目录"))?;

        if !path.exists() {
            Self::generate_default_config_file(Some(&path))?;
        }

        let content = fs::read_to_string(&path)?;
        let mut config: TaskCleanerConfig = toml::from_str(&content).unwrap_or_default();

        let mut added_count = 0;

        for bid in bundle_ids {
            let trimmed = bid.trim();
            if !trimmed.is_empty() {
                config.whitelist.disabled_rules.retain(|d| !d.eq_ignore_ascii_case(trimmed));
                if !config
                    .whitelist
                    .bundle_ids
                    .iter()
                    .any(|b| b.eq_ignore_ascii_case(trimmed))
                {
                    config.whitelist.bundle_ids.push(trimmed.to_string());
                    added_count += 1;
                }
            }
        }

        for name in names {
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                config.whitelist.disabled_rules.retain(|d| !d.eq_ignore_ascii_case(trimmed));
                if !config
                    .whitelist
                    .names
                    .iter()
                    .any(|n| n.eq_ignore_ascii_case(trimmed))
                {
                    config.whitelist.names.push(trimmed.to_string());
                    added_count += 1;
                }
            }
        }

        if added_count > 0 {
            let new_content = toml::to_string_pretty(&config)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            fs::write(&path, new_content)?;
        }

        Ok((path, added_count))
    }

    /// 自动根据传入标识（Bundle ID 或应用名称）追加至配置文件
    pub fn add_identifier_to_config(
        custom_path: Option<&Path>,
        identifier: &str,
    ) -> io::Result<(PathBuf, bool)> {
        let trimmed = identifier.trim();
        let is_bundle_id = trimmed.contains('.') && !trimmed.contains(' ');
        let (path, count) = if is_bundle_id {
            Self::append_to_user_config(custom_path, &[trimmed.to_string()], &[])?
        } else {
            Self::append_to_user_config(custom_path, &[], &[trimmed.to_string()])?
        };
        Ok((path, count > 0))
    }

    /// 自动根据传入标识（Bundle ID 或应用名称）从白名单中移除，若匹配 Core 预设则写入 disabled_rules
    pub fn remove_identifier_from_config(
        custom_path: Option<&Path>,
        identifier: &str,
    ) -> io::Result<(PathBuf, bool)> {
        let trimmed = identifier.trim();
        if Self::is_critical_system_daemon(trimmed) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("系统底层守护服务 ({}) 属于核心常驻体系，禁止从白名单移除", trimmed),
            ));
        }

        let path = custom_path
            .map(|p| p.to_path_buf())
            .or_else(Self::default_config_path)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "无法定位用户 HOME 目录"))?;

        if !path.exists() {
            Self::generate_default_config_file(Some(&path))?;
        }

        let content = fs::read_to_string(&path)?;
        let mut config: TaskCleanerConfig = toml::from_str(&content).unwrap_or_default();

        let mut modified = false;

        let initial_b_len = config.whitelist.bundle_ids.len();
        config.whitelist.bundle_ids.retain(|b| !b.eq_ignore_ascii_case(trimmed));
        if config.whitelist.bundle_ids.len() != initial_b_len {
            modified = true;
        }

        let initial_n_len = config.whitelist.names.len();
        config.whitelist.names.retain(|n| !n.eq_ignore_ascii_case(trimmed));
        if config.whitelist.names.len() != initial_n_len {
            modified = true;
        }

        if !config.whitelist.disabled_rules.iter().any(|d| d.eq_ignore_ascii_case(trimmed)) {
            config.whitelist.disabled_rules.push(trimmed.to_string());
            modified = true;
        }

        if modified {
            let new_content = toml::to_string_pretty(&config)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            fs::write(&path, new_content)?;
        }

        Ok((path, modified))
    }

    /// 更新或保存全局语言偏好到配置文件
    pub fn set_language_in_config(
        custom_path: Option<&Path>,
        lang_code: &str,
    ) -> io::Result<(PathBuf, bool)> {
        let (mut config, loaded_path) = Self::load_config(custom_path);
        let path = loaded_path
            .or_else(|| custom_path.map(|p| p.to_path_buf()))
            .or_else(Self::default_config_path)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "无法定位配置文件路径"))?;

        if !path.exists() {
            Self::generate_default_config_file(Some(&path))?;
        }

        let is_modified = if lang_code.eq_ignore_ascii_case("auto") {
            let was_some = config.general.language.is_some();
            config.general.language = None;
            was_some
        } else {
            let is_diff = config.general.language.as_deref() != Some(lang_code);
            config.general.language = Some(lang_code.to_string());
            is_diff
        };

        if is_modified {
            let new_content = toml::to_string_pretty(&config)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            fs::write(&path, new_content)?;
        }

        Ok((path, is_modified))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whitelist_l1_core_os() {
        let dummy = Path::new("/nonexistent/test/config.toml");
        let (manager, _) = WhitelistManager::new(Some(dummy), &[]);
        let finder = AppTarget {
            pid: 100,
            name: "访达".to_string(),
            bundle_id: "com.apple.finder".to_string(),
        };
        let res = manager.check_protection(&finder);
        assert!(res.is_some());
        let matched = res.unwrap();
        assert_eq!(matched.tier, WhitelistTier::L1CoreOs);
    }

    #[test]
    fn test_whitelist_l2_context_shell() {
        let dummy = Path::new("/nonexistent/test/config.toml");
        let (manager, _) = WhitelistManager::new(Some(dummy), &[]);
        let ghostty = AppTarget {
            pid: 101,
            name: "Ghostty".to_string(),
            bundle_id: "com.mitchellh.ghostty".to_string(),
        };
        let res = manager.check_protection(&ghostty);
        assert!(res.is_some());
        let matched = res.unwrap();
        assert_eq!(matched.tier, WhitelistTier::L2ContextShell);

        // 测试保护当前进程 PID
        let my_app = AppTarget {
            pid: std::process::id() as i32,
            name: "SomeApp".to_string(),
            bundle_id: "com.example.some".to_string(),
        };
        let res_pid = manager.check_protection(&my_app);
        assert!(res_pid.is_some());
        assert_eq!(res_pid.unwrap().tier, WhitelistTier::L2ContextShell);
    }

    #[test]
    fn test_whitelist_l3_utilities() {
        let dummy = Path::new("/nonexistent/test/config.toml");
        let (manager, _) = WhitelistManager::new(Some(dummy), &[]);
        let raycast = AppTarget {
            pid: 102,
            name: "Raycast".to_string(),
            bundle_id: "com.raycast.macos".to_string(),
        };
        let res = manager.check_protection(&raycast);
        assert!(res.is_some());
        assert_eq!(res.unwrap().tier, WhitelistTier::L3PersistentUtilities);
    }

    #[test]
    fn test_whitelist_l4_cli_keep() {
        let dummy = Path::new("/nonexistent/test/config.toml");
        let cli_keeps = vec!["com.google.Chrome".to_string(), "Slack".to_string()];
        let (manager, _) = WhitelistManager::new(Some(dummy), &cli_keeps);

        let chrome = AppTarget {
            pid: 103,
            name: "Google Chrome".to_string(),
            bundle_id: "com.google.Chrome".to_string(),
        };
        let res = manager.check_protection(&chrome);
        assert!(res.is_some());
        assert_eq!(res.unwrap().tier, WhitelistTier::L4CliOverride);

        let slack = AppTarget {
            pid: 104,
            name: "Slack".to_string(),
            bundle_id: "com.tinyspeck.slackmacgap".to_string(),
        };
        let res_slack = manager.check_protection(&slack);
        assert!(res_slack.is_some());
        assert_eq!(res_slack.unwrap().tier, WhitelistTier::L4CliOverride);
    }

    #[test]
    fn test_whitelist_unmatched_target() {
        let dummy = Path::new("/nonexistent/test/config.toml");
        let (manager, _) = WhitelistManager::new(Some(dummy), &[]);
        let unknown = AppTarget {
            pid: 99999,
            name: "RandomBloatwareApp".to_string(),
            bundle_id: "com.random.bloatware".to_string(),
        };
        assert!(manager.check_protection(&unknown).is_none());
    }

    #[test]
    fn test_append_to_user_config() {
        let temp_dir = std::env::temp_dir().join(format!("tc_test_{}", std::process::id()));
        let config_path = temp_dir.join("test_config.toml");

        // 第一次添加 Bundle ID
        let res1 = WhitelistManager::add_identifier_to_config(Some(&config_path), "com.spotify.client");
        assert!(res1.is_ok());
        let (_, is_new1) = res1.unwrap();
        assert!(is_new1);

        // 重复添加同一个 Bundle ID 应返回 false (已存在)
        let res2 = WhitelistManager::add_identifier_to_config(Some(&config_path), "com.spotify.client");
        assert!(res2.is_ok());
        let (_, is_new2) = res2.unwrap();
        assert!(!is_new2);

        // 添加应用名称
        let res3 = WhitelistManager::add_identifier_to_config(Some(&config_path), "网易云音乐");
        assert!(res3.is_ok());
        let (_, is_new3) = res3.unwrap();
        assert!(is_new3);

        // 验证加载
        let (config, _) = WhitelistManager::load_config(Some(&config_path));
        assert!(config.whitelist.bundle_ids.contains(&"com.spotify.client".to_string()));
        assert!(config.whitelist.names.contains(&"网易云音乐".to_string()));

        // 测试移除预设 (例如禁用 Ghostty)
        let res_remove = WhitelistManager::remove_identifier_from_config(Some(&config_path), "com.mitchellh.ghostty");
        assert!(res_remove.is_ok());

        // 验证加载后 Ghostty 是否被解除保护
        let (manager, _) = WhitelistManager::new(Some(&config_path), &[]);
        let ghostty = AppTarget {
            pid: 101,
            name: "Ghostty".to_string(),
            bundle_id: "com.mitchellh.ghostty".to_string(),
        };
        assert!(manager.check_protection(&ghostty).is_none());

        // 清理临时文件
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_cannot_remove_critical_daemons_and_can_remove_finder() {
        let temp_dir = std::env::temp_dir().join(format!("tc_crit_test_{}", std::process::id()));
        let config_path = temp_dir.join("test_config.toml");

        // Dock 是关键底层守护进程，禁止移除
        let res_dock = WhitelistManager::remove_identifier_from_config(Some(&config_path), "Dock");
        assert!(res_dock.is_err());
        assert_eq!(res_dock.unwrap_err().kind(), std::io::ErrorKind::PermissionDenied);

        // Finder 允许被移除（进入 disabled_rules）
        let res_finder = WhitelistManager::remove_identifier_from_config(Some(&config_path), "com.apple.finder");
        assert!(res_finder.is_ok());

        // 验证加载后 Finder 变为非受保护目标
        let (manager, _) = WhitelistManager::new(Some(&config_path), &[]);
        let finder_app = AppTarget {
            pid: 9999,
            name: "Finder".to_string(),
            bundle_id: "com.apple.finder".to_string(),
        };
        assert!(manager.check_protection(&finder_app).is_none());

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
