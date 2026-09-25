use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

use crate::app::{scan_foreground_apps, AppTarget};
use crate::preview::render_execution_report;
use crate::signal::tiered_terminate;
use crate::whitelist::WhitelistManager;

/// 启动交互式任务向导会话 (-i / --interactive)
pub fn run_interactive_session(
    custom_config_path: Option<&Path>,
    cli_keeps: &[String],
    grace_period: Duration,
    do_purge: bool,
) {
    let mut in_memory_keeps = cli_keeps.to_vec();

    loop {
        // 1. 初始化/刷新白名单
        let (whitelist, _) = WhitelistManager::new(custom_config_path, &in_memory_keeps);

        // 2. 扫描当前系统前台 GUI 应用
        let all_apps = scan_foreground_apps();

        let mut protected_list = Vec::new();
        let mut target_list: Vec<AppTarget> = Vec::new();

        for app in &all_apps {
            if let Some(matched) = whitelist.check_protection(app) {
                protected_list.push((app.clone(), matched));
            } else {
                target_list.push(app.clone());
            }
        }

        println!();
        println!("============================================================");
        println!("              macOS Task Cleaner - 交互式任务向导           ");
        println!("============================================================");
        println!(
            "[状态概览] 前台应用总计: {} 个 | 白名单已豁免: {} 个 | 待处置目标: {} 个",
            all_apps.len(),
            protected_list.len(),
            target_list.len()
        );
        if let Some(ref p) = whitelist.loaded_config_path {
            println!("[配置文件] {}", p.display());
        } else {
            println!("[配置文件] 内置默认规则 (未检测到外部配置文件)");
        }
        println!();

        if target_list.is_empty() {
            println!("[提示] 当前所有前台应用均已在白名单保护中，无待清理目标。");
            println!("------------------------------------------------------------");
            println!("操作指令:");
            println!("  p / protected   查看当前已被保护的应用清单");
            println!("  r / refresh     重新扫描系统前台应用");
            println!("  q / quit        退出程序");
            println!("------------------------------------------------------------");
            print!("请输入操作指令 > ");
            let _ = io::stdout().flush();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() {
                break;
            }
            let trimmed = input.trim().to_lowercase();
            match trimmed.as_str() {
                "p" | "protected" => {
                    print_protected_apps(&protected_list);
                }
                "r" | "refresh" => {
                    continue;
                }
                "q" | "quit" | "exit" => {
                    println!("[已退出] 未执行任何操作。");
                    break;
                }
                _ => {
                    println!("[提示] 输入无效，请输入 p / r / q。");
                }
            }
            continue;
        }

        // 显示待清场目标清单 (带 1-indexed 编号)
        println!("[待清场前台应用清单]:");
        println!("序号  {:<7} {:<20} {}", "PID", "应用名称", "Bundle ID");
        println!("{:-<4} {:-<7} {:-<20} {:-<30}", "", "", "", "");
        for (idx, target) in target_list.iter().enumerate() {
            println!(
                "[{:>2}] {:<7} {:<20} {}",
                idx + 1,
                target.pid,
                target.name,
                target.bundle_id
            );
        }

        println!("------------------------------------------------------------");
        println!("操作指令指南:");
        println!("  w [编号...]   将指定应用永久加入配置文件白名单 (例: w 1, 2 或 w 1 3)");
        println!("  t [编号...]   在本轮清场中临时跳过/豁免 (例: t 1)");
        println!("  c / clean     确认执行平滑清场 (向剩余未豁免目标发送 SIGTERM -> SIGKILL)");
        println!("  f / force     立即强制秒杀 (跳过宽限期，直接发送 SIGKILL)");
        println!("  p / protected 查看当前已被白名单保护的应用清单");
        println!("  r / refresh   重新扫描系统前台应用");
        println!("  q / quit      取消并安全退出");
        println!("------------------------------------------------------------");
        print!("请输入操作指令 > ");
        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        let trimmed = input.trim();
        if trimmed.is_empty() {
            continue;
        }

        // 解析指令
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let cmd = parts[0].to_lowercase();

        match cmd.as_str() {
            "q" | "quit" | "exit" => {
                println!("[已退出] 未执行任何清场操作。");
                break;
            }
            "r" | "refresh" => {
                println!("[刷新] 正在重新扫描前台应用...");
                continue;
            }
            "p" | "protected" => {
                print_protected_apps(&protected_list);
                pause_prompt();
                continue;
            }
            "c" | "clean" => {
                if confirm_action(&format!("确认平滑清场上述 {} 个应用?", target_list.len())) {
                    println!("\n[开始执行清场]...");
                    let report = tiered_terminate(&target_list, grace_period, false);
                    render_execution_report(&report, false);

                    if do_purge {
                        execute_purge();
                    }
                    break;
                } else {
                    println!("[操作已取消]");
                }
            }
            "f" | "force" => {
                if confirm_action(&format!("警告: 确认直接强制秒杀 (SIGKILL) 上述 {} 个应用?", target_list.len())) {
                    println!("\n[开始执行强制清场]...");
                    let report = tiered_terminate(&target_list, grace_period, true);
                    render_execution_report(&report, false);

                    if do_purge {
                        execute_purge();
                    }
                    break;
                } else {
                    println!("[操作已取消]");
                }
            }
            "w" | "whitelist" => {
                // 提取后续编号
                let indices = parse_indices(&parts[1..], target_list.len());
                if indices.is_empty() {
                    println!("[提示] 请指定有效序号，例如: w 1 或 w 1, 2");
                    continue;
                }

                let mut bundle_ids_to_add = Vec::new();
                let mut names_to_add = Vec::new();

                for idx in indices {
                    let target = &target_list[idx];
                    if !target.bundle_id.is_empty() {
                        bundle_ids_to_add.push(target.bundle_id.clone());
                    } else {
                        names_to_add.push(target.name.clone());
                    }
                }

                match WhitelistManager::append_to_user_config(
                    custom_config_path,
                    &bundle_ids_to_add,
                    &names_to_add,
                ) {
                    Ok((saved_path, added_count)) => {
                        println!(
                            "\n[配置更新成功] 已将 {} 个应用持久化写入白名单: {}",
                            added_count,
                            saved_path.display()
                        );
                        for bid in &bundle_ids_to_add {
                            println!("  * 包名: {}", bid);
                        }
                        for name in &names_to_add {
                            println!("  * 名称: {}", name);
                        }
                    }
                    Err(e) => {
                        eprintln!("[写入配置文件失败] {}", e);
                    }
                }
                pause_prompt();
            }
            "t" | "temp" => {
                let indices = parse_indices(&parts[1..], target_list.len());
                if indices.is_empty() {
                    println!("[提示] 请指定有效序号，例如: t 1 或 t 1, 2");
                    continue;
                }
                for idx in indices {
                    let target = &target_list[idx];
                    in_memory_keeps.push(target.bundle_id.clone());
                    println!("[临时豁免] 本轮清场将跳过: {} ({})", target.name, target.bundle_id);
                }
                pause_prompt();
            }
            _ => {
                // 检查用户是否直接输入了纯数字（如 "1" 或 "1, 2"），便捷默认等同于加入白名单
                let indices = parse_indices(&parts[..], target_list.len());
                if !indices.is_empty() {
                    println!("[快捷选择] 检测到序号输入，将其加入永久白名单:");
                    let mut bundle_ids_to_add = Vec::new();
                    let mut names_to_add = Vec::new();

                    for idx in indices {
                        let target = &target_list[idx];
                        if !target.bundle_id.is_empty() {
                            bundle_ids_to_add.push(target.bundle_id.clone());
                        } else {
                            names_to_add.push(target.name.clone());
                        }
                    }

                    match WhitelistManager::append_to_user_config(
                        custom_config_path,
                        &bundle_ids_to_add,
                        &names_to_add,
                    ) {
                        Ok((saved_path, added_count)) => {
                            println!(
                                "[配置更新成功] 已将 {} 个应用写入白名单: {}",
                                added_count,
                                saved_path.display()
                            );
                            for bid in &bundle_ids_to_add {
                                println!("  * 包名: {}", bid);
                            }
                            for name in &names_to_add {
                                println!("  * 名称: {}", name);
                            }
                        }
                        Err(e) => {
                            eprintln!("[写入配置文件失败] {}", e);
                        }
                    }
                    pause_prompt();
                } else {
                    println!("[提示] 未知指令: '{}'。请输入 w / t / c / f / p / r / q", trimmed);
                }
            }
        }
    }
}

fn print_protected_apps(protected_list: &[(AppTarget, crate::whitelist::WhitelistMatch)]) {
    println!();
    println!("------------------------------------------------------------");
    println!("[当前受保护应用清单]");
    println!("------------------------------------------------------------");
    println!("{:<7} {:<18} {:<12} {}", "PID", "应用名称", "保护层级", "豁免规则");
    println!("{:-<7} {:-<18} {:-<12} {:-<20}", "", "", "", "");
    for (app, matched) in protected_list {
        println!(
            "{:<7} {:<18} {:<12} {}",
            app.pid, app.name, matched.tier_label, matched.matched_rule
        );
    }
    println!("------------------------------------------------------------");
}

fn parse_indices(tokens: &[&str], max_len: usize) -> Vec<usize> {
    let mut indices = Vec::new();
    for token in tokens {
        // 支持逗号分隔，如 "1,2,3"
        for sub in token.split(',') {
            let s = sub.trim();
            if let Ok(num) = s.parse::<usize>() {
                if num >= 1 && num <= max_len {
                    let idx = num - 1;
                    if !indices.contains(&idx) {
                        indices.push(idx);
                    }
                }
            }
        }
    }
    indices
}

fn confirm_action(prompt: &str) -> bool {
    print!("{} [y/N]: ", prompt);
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim().to_lowercase();
        trimmed == "y" || trimmed == "yes"
    } else {
        false
    }
}

fn pause_prompt() {
    print!("\n按回车键继续...");
    let _ = io::stdout().flush();
    let mut dummy = String::new();
    let _ = io::stdin().read_line(&mut dummy);
}

fn execute_purge() {
    println!("\n[内存回收] 正在执行 /usr/sbin/purge...");
    match std::process::Command::new("/usr/sbin/purge").status() {
        Ok(status) => println!("[内存回收完成] purge 退出码: {}", status),
        Err(e) => eprintln!("[警告] 执行 purge 失败: {}", e),
    }
}
