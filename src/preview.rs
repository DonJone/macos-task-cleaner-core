use serde::{Deserialize, Serialize};

use crate::app::AppTarget;
use crate::signal::TerminationReport;
use crate::whitelist::WhitelistMatch;

/// 预检模式完整视图数据结构 (支持终端渲染与 JSON 格式化输出)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DryRunSummary {
    pub scanned_total: usize,
    pub protected_count: usize,
    pub target_count: usize,
    pub scan_duration_ms: f64,
    pub config_source: String,
    pub protected_apps: Vec<ProtectedAppEntry>,
    pub targets: Vec<TargetAppEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedAppEntry {
    pub pid: i32,
    pub name: String,
    pub bundle_id: String,
    pub tier: String,
    pub rule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetAppEntry {
    pub pid: i32,
    pub name: String,
    pub bundle_id: String,
    pub is_alive: bool,
}

/// 渲染并输出 Dry-Run 预检报表
pub fn render_dry_run_preview(
    scanned_apps: &[AppTarget],
    protected: &[(AppTarget, WhitelistMatch)],
    targets: &[AppTarget],
    scan_duration_ms: f64,
    config_path: Option<&std::path::Path>,
    as_json: bool,
) {
    let config_source = match config_path {
        Some(p) => format!("已加载配置: {}", p.display()),
        None => "内置默认规则 (无外部配置文件)".to_string(),
    };

    let mut protected_entries = Vec::new();
    for (app, matched) in protected {
        protected_entries.push(ProtectedAppEntry {
            pid: app.pid,
            name: app.name.clone(),
            bundle_id: app.bundle_id.clone(),
            tier: matched.tier_label.clone(),
            rule: matched.matched_rule.clone(),
        });
    }

    let mut target_entries = Vec::new();
    for app in targets {
        let alive = crate::signal::is_process_alive(app.pid);
        target_entries.push(TargetAppEntry {
            pid: app.pid,
            name: app.name.clone(),
            bundle_id: app.bundle_id.clone(),
            is_alive: alive,
        });
    }

    let summary = DryRunSummary {
        scanned_total: scanned_apps.len(),
        protected_count: protected.len(),
        target_count: targets.len(),
        scan_duration_ms,
        config_source,
        protected_apps: protected_entries,
        targets: target_entries,
    };

    if as_json {
        if let Ok(json_str) = serde_json::to_string_pretty(&summary) {
            println!("{}", json_str);
        }
        return;
    }

    // 纯文本友好表格排版 (无 Emoji)
    println!("============================================================");
    println!("              macOS Task Cleaner - 预检预览 (Dry-Run)       ");
    println!("============================================================");
    println!(
        "[统计概览] 发现前台图形应用: {} 个 | 白名单豁免: {} 个 | 拟清场应用: {} 个",
        summary.scanned_total, summary.protected_count, summary.target_count
    );
    println!("[配置来源] {}", summary.config_source);
    println!();

    if !summary.protected_apps.is_empty() {
        println!("------------------------------------------------------------");
        println!("[受保护应用清单 (豁免清理)]");
        println!("------------------------------------------------------------");
        println!(
            "{:<7} {:<18} {:<12} {}",
            "PID", "应用名称", "保护层级", "豁免规则匹配"
        );
        println!("{:-<7} {:-<18} {:-<12} {:-<20}", "", "", "", "");
        for item in &summary.protected_apps {
            println!(
                "{:<7} {:<18} {:<12} {}",
                item.pid, item.name, item.tier, item.rule
            );
        }
        println!();
    }

    println!("------------------------------------------------------------");
    println!("[拟终止前台应用清单 (待清场目标)]");
    println!("------------------------------------------------------------");
    if summary.targets.is_empty() {
        println!("(当前无需要清场的前台应用，工作区保持纯净)");
    } else {
        println!(
            "{:<7} {:<18} {:<30} {}",
            "PID", "应用名称", "Bundle ID", "存活状态"
        );
        println!("{:-<7} {:-<18} {:-<30} {:-<8}", "", "", "", "");
        for target in &summary.targets {
            let status_str = if target.is_alive {
                "活跃 [可发信号]"
            } else {
                "已挂起/下线"
            };
            println!(
                "{:<7} {:<18} {:<30} {}",
                target.pid, target.name, target.bundle_id, status_str
            );
        }
    }
    println!("------------------------------------------------------------");
    println!(
        "[提示] 当前为预检预览模式，未向任何目标发送实质终止信号。\n       若确认执行清场，请运行: mtc --execute (或 taskcleaner --execute)"
    );
    println!("[扫描耗时] 检索与过滤完成耗时: {:.2}ms", scan_duration_ms);
    println!("============================================================");
}

/// 渲染清场执行报告
pub fn render_execution_report(report: &TerminationReport, as_json: bool) {
    if as_json {
        if let Ok(json_str) = serde_json::to_string_pretty(report) {
            println!("{}", json_str);
        }
        return;
    }

    println!();
    println!("============================================================");
    println!("              macOS Task Cleaner - 执行结果报告             ");
    println!("============================================================");
    println!("* 目标应用总数:             {} 个", report.total_targets);
    println!("* 优雅退出 (SIGTERM 软下线): {} 个", report.terminated_sigterm);
    println!("* 兜底强退 (SIGKILL 硬回收): {} 个", report.terminated_sigkill);
    println!("* 失败/无权限数量:           {} 个", report.failed);
    println!("* 清场全流程总耗时:         {:.2}ms", report.duration_ms);
    println!("------------------------------------------------------------");

    if !report.records.is_empty() {
        println!("{:<7} {:<20} {:<14} {}", "PID", "应用名称", "退出状态", "所用信号");
        println!("{:-<7} {:-<20} {:-<14} {:-<8}", "", "", "", "");
        for rec in &report.records {
            println!(
                "{:<7} {:<20} {:<14} {}",
                rec.app.pid,
                rec.app.name,
                rec.status,
                rec.exit_signal.as_deref().unwrap_or("-")
            );
        }
    }
    println!("============================================================");
}
