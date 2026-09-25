mod app;
mod preview;
mod signal;
mod whitelist;

use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use app::scan_foreground_apps;
use preview::{render_dry_run_preview, render_execution_report};
use signal::tiered_terminate;
use whitelist::WhitelistManager;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
struct CliArgs {
    dry_run: Option<bool>,
    force: bool,
    purge: bool,
    json: bool,
    config_path: Option<PathBuf>,
    cli_keeps: Vec<String>,
    init_config: bool,
    show_help: bool,
    show_version: bool,
}

fn parse_cli_args() -> Result<CliArgs, String> {
    let mut args = env::args().skip(1);
    let mut cli = CliArgs::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-n" | "--dry-run" => {
                cli.dry_run = Some(true);
            }
            "-e" | "--execute" => {
                cli.dry_run = Some(false);
            }
            "-f" | "--force" => {
                cli.force = true;
                cli.dry_run = Some(false);
            }
            "-p" | "--purge" => {
                cli.purge = true;
            }
            "--json" => {
                cli.json = true;
            }
            "--init-config" => {
                cli.init_config = true;
            }
            "-c" | "--config" => {
                if let Some(val) = args.next() {
                    cli.config_path = Some(PathBuf::from(val));
                } else {
                    return Err("缺少 --config 参数值".to_string());
                }
            }
            "-k" | "--keep" => {
                if let Some(val) = args.next() {
                    cli.cli_keeps.push(val);
                } else {
                    return Err("缺少 --keep 参数值".to_string());
                }
            }
            "-h" | "--help" => {
                cli.show_help = true;
            }
            "-v" | "--version" => {
                cli.show_version = true;
            }
            unknown => {
                return Err(format!("未知参数: {}", unknown));
            }
        }
    }

    Ok(cli)
}

fn print_help() {
    println!("macOS Task Cleaner (taskcleaner) v{}", VERSION);
    println!("轻量级前台任务清场工具 (面向 macOS 的免弹窗、多级白名单任务清理引擎)");
    println!();
    println!("用法:");
    println!("  taskcleaner [选项]");
    println!();
    println!("核心选项:");
    println!("  -n, --dry-run             预检预览模式 (仅扫描并分析白名单过滤，不发送任何终止信号)");
    println!("  -e, --execute             执行实质清场动作 (执行 SIGTERM -> 轮询 -> SIGKILL 三段式下线)");
    println!("  -f, --force               强制直接秒杀 (跳过宽限期，直接发送 SIGKILL)");
    println!("  -k, --keep <NAME/BUNDLE>  命令行临时追加豁免白名单 (支持多次传入)");
    println!("  -p, --purge               清场完成后调用 /usr/sbin/purge 强制回收内存缓存");
    println!("  -c, --config <FILE>       指定自定义 TOML 配置文件路径");
    println!("      --init-config         在 ~/.config/taskcleaner/config.toml 生成默认配置模板");
    println!("      --json                以结构化 JSON 格式输出结果 (适配 Raycast / 脚本接入)");
    println!("  -h, --help                显示帮助说明");
    println!("  -v, --version             显示当前版本");
    println!();
    println!("白名单分级体系:");
    println!("  L1: 系统核心层 (Finder, Dock, WindowServer)");
    println!("  L2: 会话终端层 (保护当前调用终端、父会话 PID 与常用终端/IDE)");
    println!("  L3: 常驻设施层 (Raycast, Alfred, 窗口管理与输入法)");
    println!("  L4: 用户配置层 (来自配置文件与 -k/--keep 命令行参数)");
}

fn main() {
    let cli = match parse_cli_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("[错误] {}", e);
            eprintln!("请使用 --help 查看完整参数说明。");
            std::process::exit(1);
        }
    };

    if cli.show_help {
        print_help();
        return;
    }

    if cli.show_version {
        println!("taskcleaner v{}", VERSION);
        return;
    }

    // 处理配置文件初始化
    if cli.init_config {
        match WhitelistManager::generate_default_config_file(cli.config_path.as_deref()) {
            Ok(path) => {
                println!("[配置初始化成功] 配置文件已生成至: {}", path.display());
                return;
            }
            Err(e) => {
                eprintln!("[配置初始化失败] 无法写入配置文件: {}", e);
                std::process::exit(1);
            }
        }
    }

    let scan_start = Instant::now();

    // 1. 初始化白名单与用户配置
    let (whitelist, config) =
        WhitelistManager::new(cli.config_path.as_deref(), &cli.cli_keeps);

    // 确定运行模式：CLI 显式指定 > 配置文件指定 > 默认 true
    let is_dry_run = cli
        .dry_run
        .unwrap_or(config.general.default_dry_run);

    let grace_period = Duration::from_millis(config.general.grace_period_ms);

    // 2. 扫描前台 GUI 应用
    let scanned_apps = scan_foreground_apps();

    // 3. 应用白名单多级过滤网
    let mut protected_list = Vec::new();
    let mut target_list = Vec::new();

    for app in scanned_apps.iter() {
        if let Some(matched) = whitelist.check_protection(app) {
            protected_list.push((app.clone(), matched));
        } else {
            target_list.push(app.clone());
        }
    }

    let scan_duration_ms = scan_start.elapsed().as_secs_f64() * 1000.0;

    // 4. 预检模式 (Dry-Run)
    if is_dry_run {
        render_dry_run_preview(
            &scanned_apps,
            &protected_list,
            &target_list,
            scan_duration_ms,
            whitelist.loaded_config_path.as_deref(),
            cli.json,
        );
        return;
    }

    // 5. 实质执行清场
    let report = tiered_terminate(&target_list, grace_period, cli.force);
    render_execution_report(&report, cli.json);

    // 6. 可选内存整理 (--purge)
    if cli.purge {
        if !cli.json {
            println!("\n[内存回收] 正在执行 /usr/sbin/purge 回收 inactive 页面...");
        }
        match Command::new("/usr/sbin/purge").status() {
            Ok(status) => {
                if !cli.json {
                    println!("[内存回收完成] purge 退出码: {}", status);
                }
            }
            Err(e) => {
                eprintln!("[警告] 执行 /usr/sbin/purge 失败: {}", e);
            }
        }
    }
}
