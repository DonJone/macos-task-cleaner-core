use std::collections::HashSet;
use std::io::{self, ErrorKind};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::app::{terminate_via_appkit, AppTarget};
use crate::whitelist::WhitelistManager;

pub const PROC_PIDT_SHORTBSDINFO: libc::c_int = 13;
pub const SZOMB: u32 = 5;
pub const PROC_FLAG_INEXIT: u32 = 4;

/// macOS native libproc process short info structure
#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
pub struct ProcBsdShortInfo {
    pub pbsi_pid: u32,
    pub pbsi_ppid: u32,
    pub pbsi_pgid: u32,
    pub pbsi_status: u32,
    pub pbsi_comm: [u8; 16],
    pub pbsi_flags: u32,
    pub pbsi_uid: libc::uid_t,
    pub pbsi_gid: libc::gid_t,
    pub pbsi_ruid: libc::uid_t,
    pub pbsi_rgid: libc::gid_t,
    pub pbsi_svuid: libc::uid_t,
    pub pbsi_svgid: libc::gid_t,
    pub pbsi_rfu: u32,
}

unsafe extern "C" {
    fn proc_pidinfo(
        pid: libc::c_int,
        flavor: libc::c_int,
        arg: u64,
        buffer: *mut libc::c_void,
        buffersize: libc::c_int,
    ) -> libc::c_int;
}

/// 获取指定进程的父进程 PID (PPID)
pub fn get_process_parent_pid(pid: i32) -> Option<i32> {
    if pid <= 1 {
        return None;
    }
    let mut info = ProcBsdShortInfo::default();
    let size = std::mem::size_of::<ProcBsdShortInfo>() as libc::c_int;
    let ret = unsafe {
        proc_pidinfo(
            pid as libc::c_int,
            PROC_PIDT_SHORTBSDINFO,
            0,
            &mut info as *mut _ as *mut libc::c_void,
            size,
        )
    };
    if ret == size && info.pbsi_ppid > 0 {
        Some(info.pbsi_ppid as i32)
    } else {
        None
    }
}

/// 递归解析调用者会话链路上的全部祖先进程 PID 集合（直溯 launchd / PID 1）
/// 用于构建高防护度的 L2 防御屏障，杜绝任何误伤终端、tmux、IDE 或宿主 GUI 的可能
pub fn get_caller_lineage() -> HashSet<i32> {
    let mut lineage = HashSet::new();
    let my_pid = std::process::id() as i32;
    lineage.insert(my_pid);

    let mut current = my_pid;
    // 限制最大溯源层深为 32，防止环状异常
    for _ in 0..32 {
        if let Some(ppid) = get_process_parent_pid(current) {
            if ppid <= 1 || lineage.contains(&ppid) {
                if ppid > 0 {
                    lineage.insert(ppid);
                }
                break;
            }
            lineage.insert(ppid);
            current = ppid;
        } else {
            break;
        }
    }

    // 兜底补齐 libc::getppid()
    let direct_ppid = unsafe { libc::getppid() };
    if direct_ppid > 0 {
        lineage.insert(direct_ppid);
    }

    lineage
}

/// 判定目标是否为 Finder / 访达
pub fn is_finder(identifier: &str) -> bool {
    let trimmed = identifier.trim();
    trimmed.eq_ignore_ascii_case("com.apple.finder")
        || trimmed.eq_ignore_ascii_case("Finder")
        || trimmed == "访达"
}

/// 进程终止状态强类型枚举 (Unix 规范状态分类)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TerminationStatusCode {
    #[default]
    Unknown,
    /// POSIX SIGTERM (15) 退出成功
    SuccessSigterm,
    /// POSIX SIGKILL (9) 强制终止成功
    SuccessSigkill,
    /// AppKit 原生 NSRunningApplication.terminate 正常退出成功
    SuccessAppKit,
    /// 调用者祖先会话链路保护，安全跳过
    SkippedCallerLineage,
    /// 系统底层核心守护进程保护，安全跳过
    SkippedCriticalDaemon,
    /// POSIX 信号派发权限拒绝 (EPERM)
    FailedPermissionDenied,
    /// POSIX 信号派发失败
    FailedDispatch,
    /// 目标无响应且超时
    FailedTimeout,
    /// 强制终止失败 (SIGKILL 失败)
    FailedKill,
}

/// 单个应用的终止处置明细
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessTerminationRecord {
    pub app: AppTarget,
    pub status: String,
    #[serde(default)]
    pub status_code: TerminationStatusCode,
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
    #[serde(default)]
    pub cache_purged: bool,
}

/// 系统清理模式定义 (对齐 GUI 规范与 POSIX 语义)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TerminationMode {
    /// 标准梯次终止: SIGTERM -> 轮询宽限期 -> SIGKILL 兜底
    #[default]
    Standard,
    /// 强制直接终止: 立即 SIGKILL (对 Finder 仍使用 AppKit voluntary quit 免于闪回)
    ForceImmediate,
    /// 标准梯次终止并在完成后调用 /usr/sbin/purge 整理系统内存
    StandardWithPurge,
}

/// 执行系统内存缓存清理 (/usr/sbin/purge)
/// 回收 inactive 文件缓存页面，对应 GUI "终止并清空系统缓存" 模式
pub fn purge_system_cache() -> io::Result<()> {
    let output = std::process::Command::new("/usr/sbin/purge").output()?;
    if output.status.success() {
        Ok(())
    } else {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "purge 执行失败 (退出码 {:?}): {}",
                output.status.code(),
                err_msg.trim()
            ),
        ))
    }
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

/// 检测进程是否依然存活
/// 融合 libproc 深度状态探查与 POSIX kill(pid, 0) 无损探测
/// 精准识别处于退出过渡期 (PROC_FLAG_INEXIT) 或等待父进程回收的僵尸进程 (SZOMB)
pub fn is_process_alive(pid: i32) -> bool {
    if pid <= 0 {
        return false;
    }

    let mut info = ProcBsdShortInfo::default();
    let size = std::mem::size_of::<ProcBsdShortInfo>() as libc::c_int;
    let ret = unsafe {
        proc_pidinfo(
            pid as libc::c_int,
            PROC_PIDT_SHORTBSDINFO,
            0,
            &mut info as *mut _ as *mut libc::c_void,
            size,
        )
    };

    if ret == size {
        // 5 = SZOMB (僵尸进程，已退出但尚未被父进程 wait 收集)
        // 4 = PROC_FLAG_INEXIT (进程正在执行 exit 系统调用退场)
        if info.pbsi_status == SZOMB || (info.pbsi_flags & PROC_FLAG_INEXIT != 0) {
            return false;
        }
        return true;
    }

    // 降级使用 POSIX kill(pid, 0) 无损探测
    let kill_ret = unsafe { libc::kill(pid, 0) };
    if kill_ret == 0 {
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
    tiered_terminate_with_lang(targets, grace_period, force_immediate, crate::i18n::Language::ZhHans)
}

/// 支持指定国际化语言的分级安全终止引擎
pub fn tiered_terminate_with_lang(
    targets: &[AppTarget],
    grace_period: Duration,
    force_immediate: bool,
    lang: crate::i18n::Language,
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
            cache_purged: false,
        };
    }

    // 核心安全前置过滤：绝对禁止向当前调用者会话链路或底层核心守护进程派发信号
    let caller_lineage = get_caller_lineage();

    let mut valid_targets = Vec::new();
    for target in targets {
        if caller_lineage.contains(&target.pid) {
            failed += 1;
            records.push(ProcessTerminationRecord {
                app: target.clone(),
                status: TerminationStatusCode::SkippedCallerLineage.localized_description(lang).to_string(),
                status_code: TerminationStatusCode::SkippedCallerLineage,
                exit_signal: None,
                error_msg: Some(crate::i18n::CoreMessages::caller_lineage_error(target.pid, lang)),
            });
        } else if WhitelistManager::is_critical_system_daemon(&target.bundle_id)
            || WhitelistManager::is_critical_system_daemon(&target.name)
        {
            failed += 1;
            records.push(ProcessTerminationRecord {
                app: target.clone(),
                status: TerminationStatusCode::SkippedCriticalDaemon.localized_description(lang).to_string(),
                status_code: TerminationStatusCode::SkippedCriticalDaemon,
                exit_signal: None,
                error_msg: Some(crate::i18n::CoreMessages::critical_daemon_error(lang).to_string()),
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
                        status: TerminationStatusCode::SuccessAppKit.localized_description(lang).to_string(),
                        status_code: TerminationStatusCode::SuccessAppKit,
                        exit_signal: Some("NSApplicationTerminate".to_string()),
                        error_msg: None,
                    });
                    continue;
                } else {
                    failed += 1;
                    records.push(ProcessTerminationRecord {
                        app: target.clone(),
                        status: TerminationStatusCode::FailedTimeout.localized_description(lang).to_string(),
                        status_code: TerminationStatusCode::FailedTimeout,
                        exit_signal: Some("NSApplicationTerminate".to_string()),
                        error_msg: Some(crate::i18n::CoreMessages::finder_timeout_error(lang).to_string()),
                    });
                    continue;
                }
            }

            match send_posix_signal(target.pid, libc::SIGKILL) {
                Ok(_) => {
                    terminated_sigkill += 1;
                    records.push(ProcessTerminationRecord {
                        app: target.clone(),
                        status: TerminationStatusCode::SuccessSigkill.localized_description(lang).to_string(),
                        status_code: TerminationStatusCode::SuccessSigkill,
                        exit_signal: Some("SIGKILL".to_string()),
                        error_msg: None,
                    });
                }
                Err(e) => {
                    failed += 1;
                    records.push(ProcessTerminationRecord {
                        app: target.clone(),
                        status: TerminationStatusCode::FailedKill.localized_description(lang).to_string(),
                        status_code: TerminationStatusCode::FailedKill,
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
            cache_purged: false,
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
                            status: TerminationStatusCode::FailedDispatch.localized_description(lang).to_string(),
                            status_code: TerminationStatusCode::FailedDispatch,
                            exit_signal: None,
                            error_msg: Some(e.to_string()),
                        });
                    }
                }
            } else {
                terminated_sigterm += 1;
                records.push(ProcessTerminationRecord {
                    app: target.clone(),
                    status: TerminationStatusCode::SuccessAppKit.localized_description(lang).to_string(),
                    status_code: TerminationStatusCode::SuccessAppKit,
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
                        status: TerminationStatusCode::FailedPermissionDenied.localized_description(lang).to_string(),
                        status_code: TerminationStatusCode::FailedPermissionDenied,
                        exit_signal: Some("SIGTERM".to_string()),
                        error_msg: Some(e.to_string()),
                    });
                }
                Err(_) => {
                    // 派发瞬间可能已退出
                    terminated_sigterm += 1;
                    records.push(ProcessTerminationRecord {
                        app: target.clone(),
                        status: TerminationStatusCode::SuccessSigterm.localized_description(lang).to_string(),
                        status_code: TerminationStatusCode::SuccessSigterm,
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
                let (exit_sig, code) = if is_finder(&item.bundle_id) || is_finder(&item.name) {
                    ("NSApplicationTerminate", TerminationStatusCode::SuccessAppKit)
                } else {
                    ("SIGTERM", TerminationStatusCode::SuccessSigterm)
                };
                records.push(ProcessTerminationRecord {
                    app: item,
                    status: code.localized_description(lang).to_string(),
                    status_code: code,
                    exit_signal: Some(exit_sig.to_string()),
                    error_msg: None,
                });
            }
        }
        pending_targets = still_alive;
    }

    // 阶段三：对剩余依然处于挂起/死锁的顽固进程，发送 SIGKILL (kill -9) 兜底
    for stubborn_app in pending_targets {
        // 对 Finder 特殊保护：禁止直接 SIGKILL，避免 launchd 判定异常而反复闪回拉起
        if is_finder(&stubborn_app.bundle_id) || is_finder(&stubborn_app.name) {
            terminate_via_appkit(stubborn_app.pid);
            if !is_process_alive(stubborn_app.pid) {
                terminated_sigterm += 1;
                records.push(ProcessTerminationRecord {
                    app: stubborn_app,
                    status: TerminationStatusCode::SuccessAppKit.localized_description(lang).to_string(),
                    status_code: TerminationStatusCode::SuccessAppKit,
                    exit_signal: Some("NSApplicationTerminate".to_string()),
                    error_msg: None,
                });
            } else {
                failed += 1;
                records.push(ProcessTerminationRecord {
                    app: stubborn_app,
                    status: TerminationStatusCode::FailedTimeout.localized_description(lang).to_string(),
                    status_code: TerminationStatusCode::FailedTimeout,
                    exit_signal: Some("NSApplicationTerminate".to_string()),
                    error_msg: Some(crate::i18n::CoreMessages::finder_timeout_error(lang).to_string()),
                });
            }
            continue;
        }

        // 预防 PID 复用竞态：在派发 SIGKILL 前核验进程是否实际存活
        if !is_process_alive(stubborn_app.pid) {
            terminated_sigterm += 1;
            records.push(ProcessTerminationRecord {
                app: stubborn_app,
                status: TerminationStatusCode::SuccessSigterm.localized_description(lang).to_string(),
                status_code: TerminationStatusCode::SuccessSigterm,
                exit_signal: Some("SIGTERM".to_string()),
                error_msg: None,
            });
            continue;
        }

        match send_posix_signal(stubborn_app.pid, libc::SIGKILL) {
            Ok(_) => {
                terminated_sigkill += 1;
                records.push(ProcessTerminationRecord {
                    app: stubborn_app,
                    status: TerminationStatusCode::SuccessSigkill.localized_description(lang).to_string(),
                    status_code: TerminationStatusCode::SuccessSigkill,
                    exit_signal: Some("SIGKILL".to_string()),
                    error_msg: None,
                });
            }
            Err(e) => {
                failed += 1;
                records.push(ProcessTerminationRecord {
                    app: stubborn_app,
                    status: TerminationStatusCode::FailedKill.localized_description(lang).to_string(),
                    status_code: TerminationStatusCode::FailedKill,
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
        cache_purged: false,
    }
}

/// 按照指定的清理模式执行终止流程 (支持指定语言)
pub fn terminate_with_mode_with_lang(
    targets: &[AppTarget],
    grace_period: Duration,
    mode: TerminationMode,
    lang: crate::i18n::Language,
) -> TerminationReport {
    match mode {
        TerminationMode::Standard => tiered_terminate_with_lang(targets, grace_period, false, lang),
        TerminationMode::ForceImmediate => tiered_terminate_with_lang(targets, grace_period, true, lang),
        TerminationMode::StandardWithPurge => {
            let mut rep = tiered_terminate_with_lang(targets, grace_period, false, lang);
            if let Ok(()) = purge_system_cache() {
                rep.cache_purged = true;
            }
            rep
        }
    }
}

/// 按照指定的清理模式执行终止流程 (默认中文，保留向后兼容)
pub fn terminate_with_mode(
    targets: &[AppTarget],
    grace_period: Duration,
    mode: TerminationMode,
) -> TerminationReport {
    terminate_with_mode_with_lang(targets, grace_period, mode, crate::i18n::Language::ZhHans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_caller_lineage_resolution() {
        let lineage = get_caller_lineage();
        let my_pid = std::process::id() as i32;
        assert!(lineage.contains(&my_pid), "Lineage must contain self PID");
        let my_ppid = unsafe { libc::getppid() };
        assert!(lineage.contains(&my_ppid), "Lineage must contain direct PPID");
    }

    #[test]
    fn test_is_process_alive() {
        let my_pid = std::process::id() as i32;
        assert!(is_process_alive(my_pid), "Self process must be alive");
        assert!(!is_process_alive(-1), "Negative PID must be dead");
        assert!(!is_process_alive(999_999_999), "Non-existent PID must be dead");
    }

    #[test]
    fn test_tiered_terminate_lineage_protection() {
        let my_pid = std::process::id() as i32;
        let dummy_target = AppTarget {
            pid: my_pid,
            name: "SelfProcess".to_string(),
            bundle_id: "com.example.self".to_string(),
        };

        let report = tiered_terminate(&[dummy_target], Duration::from_millis(50), false);
        assert_eq!(report.failed, 1);
        assert_eq!(report.terminated_sigterm, 0);
        assert_eq!(report.terminated_sigkill, 0);
        assert_eq!(
            report.records[0].status_code,
            TerminationStatusCode::SkippedCallerLineage
        );
    }

    #[test]
    fn test_tiered_terminate_critical_daemon_protection() {
        let dock_target = AppTarget {
            pid: 99999,
            name: "Dock".to_string(),
            bundle_id: "com.apple.dock".to_string(),
        };

        let report = tiered_terminate(&[dock_target], Duration::from_millis(50), false);
        assert_eq!(report.failed, 1);
        assert_eq!(report.records[0].status_code, TerminationStatusCode::SkippedCriticalDaemon);
    }
}
