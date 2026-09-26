use objc2_app_kit::{NSApplicationActivationPolicy, NSRunningApplication, NSWorkspace};
use serde::{Deserialize, Serialize};

pub fn terminate_via_appkit(pid: i32) -> bool {
    if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
        app.terminate()
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminate_via_appkit_finder() {
        // If Finder is running, test terminating it cleanly
        let apps = scan_foreground_apps();
        if let Some(finder) = apps.iter().find(|a| a.bundle_id == "com.apple.finder") {
            let ok = terminate_via_appkit(finder.pid);
            assert!(ok);
            std::thread::sleep(std::time::Duration::from_millis(300));
            // Ensure Finder is NOT alive
            let alive = crate::signal::is_process_alive(finder.pid);
            assert!(!alive, "Finder should be completely terminated without respawning");
        }
    }
}

/// 目标进程结构定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTarget {
    pub pid: i32,
    pub name: String,
    pub bundle_id: String,
}

/// 扫描系统中所有前台图形应用 (FR-1)
/// 严格限定为 Dock 栏与 Cmd+Tab 切换器可见的活跃图形界面应用，自动排除纯后台与小工具进程
pub fn scan_foreground_apps() -> Vec<AppTarget> {
    let workspace = NSWorkspace::sharedWorkspace();
    let running_apps = workspace.runningApplications();
    let mut targets = Vec::new();

    for app in &running_apps {
        // 仅匹配在 Dock 栏与 Cmd+Tab 可见的常规图形应用
        if app.activationPolicy() != NSApplicationActivationPolicy::Regular {
            continue;
        }

        let pid = app.processIdentifier() as i32;
        // 过滤已终止、挂起或无效的负值 PID
        if pid <= 0 {
            continue;
        }

        let bundle_id = app
            .bundleIdentifier()
            .map(|s| s.to_string())
            .unwrap_or_default();

        let name = app
            .localizedName()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        targets.push(AppTarget {
            pid,
            name,
            bundle_id,
        });
    }

    // 按应用名称字母序排序，保持输出稳定
    targets.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    targets
}
