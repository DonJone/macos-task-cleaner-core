use std::io::{self, ErrorKind};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::app::{terminate_via_appkit, AppTarget};
use crate::whitelist::WhitelistManager;

/// 判定目标是否为 Finder / 访达
pub fn is_finder(identifier: &str) -> bool {
    let trimmed = identifier.trim();
    trimmed.eq_ignore_ascii_case("com.apple.finder")
        || trimmed.eq_ignore_ascii_case("Finder")
        || trimmed == "访达"
}

/// 单个应用的终止处置明细
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessTerminationRecord {
    pub app: AppTarget,
    pub status: String,
    pub exit_signal: Option<String>,
    pub error_msg: Option<String>,
}

/// 清场执行报告汇总
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerminationReport {
    pub total_targets: usize,
    pub terminated_sigterm: usize,
    pub terminated_sigkill: usize,
    pub failed: usize,
    pub duration_ms: f64,
    pub records: Vec<ProcessTerminationRecord>,
}

/// 发送 POSIX 信号至指定 PID
pub fn send_posix_signal(pid: i32, signal: i32) -> io::Result<()> {
    let ret = unsafe { libc::kill(pid, signal) };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// 检测进程是否依然存活 (发送无损探测信号 0)
pub fn is_process_alive(pid: i32) -> bool {
    let ret = unsafe { libc::kill(pid, 0) };
    if ret == 0 {
        true
    } else {
        let err = io::Error::last_os_error();
        // EPERM 说明进程存在，仅当前无权发信；ESRCH 说明进程已彻底不存在
        err.raw_os_error() == Some(libc::EPERM)
    }
}

/// 执行三段式降级清场核心引擎 (FR-2, FR-3)
pub fn tiered_terminate(
    targets: &[AppTarget],
    grace_period: Duration,
    force_immediate: bool,
) -> TerminationReport {
    let start = Instant::now();
    let mut records = Vec::new();
    let mut terminated_sigterm = 0;
    let mut terminated_sigkill = 0;
    let mut failed = 0;

    if targets.is_empty() {
        return TerminationReport {
            total_targets: 0,
            terminated_sigterm: 0,
            terminated_sigkill: 0,
            failed: 0,
            duration_ms: start.elapsed().as_secs_f64() * 1000.0,
            records,
        };
    }

    // 核心安全前置过滤：绝对禁止向当前调用者会话或底层核心守护进程（如 WindowServer, loginwindow, Dock）派发信号
    let my_pid = std::process::id() as i32;
    let my_ppid = unsafe { libc::getppid() };

    let mut valid_targets = Vec::new();
    for target in targets {
        if target.pid == my_pid || target.pid == my_ppid {
            failed += 1;
            records.push(ProcessTerminationRecord {
                app: target.clone(),
                status: "保护调用者会话，跳过终止".to_string(),
                exit_signal: None,
                error_msg: Some("当前执行会话或父进程处于受保护状态".to_string()),
            });
        } else if WhitelistManager::is_critical_system_daemon(&target.bundle_id)
            || WhitelistManager::is_critical_system_daemon(&target.name)
        {
            failed += 1;
            records.push(ProcessTerminationRecord {
                app: target.clone(),
                status: "系统底层守护进程受常驻保护，已跳过终止".to_string(),
                exit_signal: None,
                error_msg: Some("系统底层关键守护服务禁止强制终止".to_string()),
            });
        } else {
            valid_targets.push(target.clone());
        }
    }

    if force_immediate {
        // --force 模式：跳过 SIGTERM 与轮询宽限期
        for target in &valid_targets {
            if is_finder(&target.bundle_id) || is_finder(&target.name) {
                // 对 Finder 而言，直接发送 SIGKILL 会被 launchd 判定为异常崩溃而立即拉起（闪回）。
                // 因此即使在 force 模式下，亦优先派发 AppKit 原生 terminate() 正常退出指令。
                if terminate_via_appkit(target.pid) {
                    let wait_start = Instant::now();
                    while wait_start.elapsed() < Duration::from_millis(250) {
                        if !is_process_alive(target.pid) {
                            break;
                        }
                        thread::sleep(Duration::from_millis(20));
                    }
                }
                if !is_process_alive(target.pid) {
                    terminated_sigkill += 1;
                    records.push(ProcessTerminationRecord {
                        app: target.clone(),
                        status: "终止成功".to_string(),
                        exit_signal: Some("NSApplicationTerminate".to_string()),
                        error_msg: None,
                    });
                    continue;
                }
            }

            match send_posix_signal(target.pid, libc::SIGKILL) {
                Ok(_) => {
                    terminated_sigkill += 1;
                    records.push(ProcessTerminationRecord {
                        app: target.clone(),
                        status: "强制终止成功 (SIGKILL)".to_string(),
                        exit_signal: Some("SIGKILL".to_string()),
                        error_msg: None,
                    });
                }
                Err(e) => {
                    failed += 1;
                    records.push(ProcessTerminationRecord {
                        app: target.clone(),
                        status: "强制终止失败 (SIGKILL)".to_string(),
                        exit_signal: Some("SIGKILL".to_string()),
                        error_msg: Some(e.to_string()),
                    });
                }
            }
        }

        return TerminationReport {
            total_targets: targets.len(),
            terminated_sigterm,
            terminated_sigkill,
            failed,
            duration_ms: start.elapsed().as_secs_f64() * 1000.0,
            records,
        };
    }

    // 阶段一：批量派发软信号 / 请求
    let mut pending_targets: Vec<AppTarget> = Vec::new();

    for target in &valid_targets {
        if is_finder(&target.bundle_id) || is_finder(&target.name) {
            // 对 Finder 而言，通过 AppKit 原生 terminate() 触发正常退出流程，通知 launchd 免于 KeepAlive 重新拉起
            if terminate_via_appkit(target.pid) {
                pending_targets.push(target.clone());
            } else if is_process_alive(target.pid) {
                match send_posix_signal(target.pid, libc::SIGTERM) {
                    Ok(_) => pending_targets.push(target.clone()),
                    Err(e) => {
                        failed += 1;
                        records.push(ProcessTerminationRecord {
                            app: target.clone(),
                            status: "退出派发失败".to_string(),
                            exit_signal: None,
                            error_msg: Some(e.to_string()),
                        });
                    }
                }
            } else {
                terminated_sigterm += 1;
                records.push(ProcessTerminationRecord {
                    app: target.clone(),
                    status: "终止成功".to_string(),
                    exit_signal: Some("NSApplicationTerminate".to_string()),
                    error_msg: None,
                });
            }
        } else {
            match send_posix_signal(target.pid, libc::SIGTERM) {
                Ok(_) => {
                    pending_targets.push(target.clone());
                }
                Err(e) if e.kind() == ErrorKind::PermissionDenied => {
                    failed += 1;
                    records.push(ProcessTerminationRecord {
                        app: target.clone(),
                        status: "权限拒绝".to_string(),
                        exit_signal: Some("SIGTERM".to_string()),
                        error_msg: Some(e.to_string()),
                    });
                }
                Err(_) => {
                    // 派发瞬间可能已退出
                    terminated_sigterm += 1;
                    records.push(ProcessTerminationRecord {
                        app: target.clone(),
                        status: "终止成功".to_string(),
                        exit_signal: Some("SIGTERM".to_string()),
                        error_msg: None,
                    });
                }
            }
        }
    }

    // 阶段二：宽限期高频轮询 (默认 25ms 轮询一次)
    let poll_interval = Duration::from_millis(25);
    let wait_start = Instant::now();

    while !pending_targets.is_empty() && wait_start.elapsed() < grace_period {
        thread::sleep(poll_interval);

        let mut still_alive = Vec::new();
        for item in pending_targets {
            if is_process_alive(item.pid) {
                still_alive.push(item);
            } else {
                terminated_sigterm += 1;
                let exit_sig = if is_finder(&item.bundle_id) || is_finder(&item.name) {
                    "NSApplicationTerminate"
                } else {
                    "SIGTERM"
                };
                records.push(ProcessTerminationRecord {
                    app: item,
                    status: "终止成功".to_string(),
                    exit_signal: Some(exit_sig.to_string()),
                    error_msg: None,
                });
            }
        }
        pending_targets = still_alive;
    }

    // 阶段三：对剩余依然处于挂起/死锁的顽固进程，发送 SIGKILL (kill -9) 兜底
    for stubborn_app in pending_targets {
        match send_posix_signal(stubborn_app.pid, libc::SIGKILL) {
            Ok(_) => {
                terminated_sigkill += 1;
                records.push(ProcessTerminationRecord {
                    app: stubborn_app,
                    status: "强制终止成功 (SIGKILL)".to_string(),
                    exit_signal: Some("SIGKILL".to_string()),
                    error_msg: None,
                });
            }
            Err(e) => {
                failed += 1;
                records.push(ProcessTerminationRecord {
                    app: stubborn_app,
                    status: "强制终止失败 (SIGKILL)".to_string(),
                    exit_signal: Some("SIGKILL".to_string()),
                    error_msg: Some(e.to_string()),
                });
            }
        }
    }

    TerminationReport {
        total_targets: targets.len(),
        terminated_sigterm,
        terminated_sigkill,
        failed,
        duration_ms: start.elapsed().as_secs_f64() * 1000.0,
        records,
    }
}
