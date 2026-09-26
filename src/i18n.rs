use serde::{Deserialize, Serialize};

use crate::signal::TerminationStatusCode;
use crate::whitelist::WhitelistTier;

/// 支持的国际化语言枚举 (与 GUI 客户端保持一致支持 24 种语言)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Language {
    #[default]
    En,
    ZhHans,
    ZhHant,
    Ja,
    Ko,
    Fr,
    De,
    Es,
    Pt,
    It,
    Ru,
    Nl,
    Pl,
    Tr,
    Ar,
    Th,
    Vi,
    Id,
    Sv,
    Da,
    Nb,
    Fi,
    Cs,
    Uk,
}

impl Language {
    /// 从语言标识符 (形如 "zh-CN", "en_US", "ja", "zh-Hans") 解析 Language
    pub fn from_locale_str(s: &str) -> Self {
        let s = s.to_lowercase().replace('_', "-");
        if s.starts_with("zh-hant")
            || s.starts_with("zh-tw")
            || s.starts_with("zh-hk")
            || s.starts_with("zh-mo")
            || s.starts_with("zh-cht")
        {
            Self::ZhHant
        } else if s.starts_with("zh") {
            Self::ZhHans
        } else if s.starts_with("ja") {
            Self::Ja
        } else if s.starts_with("ko") {
            Self::Ko
        } else if s.starts_with("fr") {
            Self::Fr
        } else if s.starts_with("de") {
            Self::De
        } else if s.starts_with("es") {
            Self::Es
        } else if s.starts_with("pt") {
            Self::Pt
        } else if s.starts_with("it") {
            Self::It
        } else if s.starts_with("ru") {
            Self::Ru
        } else if s.starts_with("nl") {
            Self::Nl
        } else if s.starts_with("pl") {
            Self::Pl
        } else if s.starts_with("tr") {
            Self::Tr
        } else if s.starts_with("ar") {
            Self::Ar
        } else if s.starts_with("th") {
            Self::Th
        } else if s.starts_with("vi") {
            Self::Vi
        } else if s.starts_with("id") {
            Self::Id
        } else if s.starts_with("sv") {
            Self::Sv
        } else if s.starts_with("da") {
            Self::Da
        } else if s.starts_with("nb") || s.starts_with("no") {
            Self::Nb
        } else if s.starts_with("fi") {
            Self::Fi
        } else if s.starts_with("cs") {
            Self::Cs
        } else if s.starts_with("uk") {
            Self::Uk
        } else {
            Self::En
        }
    }

    /// 自动检测系统/终端当前语言偏好 (优先遵循终端 LC_ALL/LC_MESSAGES/LANG 环境变量，其次回退至 macOS 全局 AppleLanguages)
    pub fn detect_system() -> Self {
        // 1. 优先检查终端环境变量 (LC_ALL -> LC_MESSAGES -> LANG)
        if let Ok(lang) = std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LC_MESSAGES"))
            .or_else(|_| std::env::var("LANG"))
        {
            let trimmed = lang.trim();
            if !trimmed.is_empty() && trimmed != "C" && trimmed != "POSIX" {
                return Self::from_locale_str(trimmed);
            }
        }

        // 2. 回退从 macOS defaults read -g AppleLanguages 读取系统全局偏好
        if let Ok(output) = std::process::Command::new("/usr/bin/defaults")
            .args(["read", "-g", "AppleLanguages"])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let trimmed = line.trim().trim_matches(|c| {
                        c == '(' || c == ')' || c == ',' || c == '"' || c == ' '
                    });
                    if !trimmed.is_empty() && !trimmed.starts_with('(') {
                        return Self::from_locale_str(trimmed);
                    }
                }
            }
        }

        Self::En
    }

    /// 获取 BCP-47 标准语言代码
    pub fn code(&self) -> &'static str {
        match self {
            Self::En => "en",
            Self::ZhHans => "zh-Hans",
            Self::ZhHant => "zh-Hant",
            Self::Ja => "ja",
            Self::Ko => "ko",
            Self::Fr => "fr",
            Self::De => "de",
            Self::Es => "es",
            Self::Pt => "pt",
            Self::It => "it",
            Self::Ru => "ru",
            Self::Nl => "nl",
            Self::Pl => "pl",
            Self::Tr => "tr",
            Self::Ar => "ar",
            Self::Th => "th",
            Self::Vi => "vi",
            Self::Id => "id",
            Self::Sv => "sv",
            Self::Da => "da",
            Self::Nb => "nb",
            Self::Fi => "fi",
            Self::Cs => "cs",
            Self::Uk => "uk",
        }
    }
}

/// 本地化文案转换函数
impl WhitelistTier {
    /// 语言无关的机器可读标识符 (例如 "l1_core_os", "l2_context_shell")
    pub fn identifier(&self) -> &'static str {
        match self {
            Self::L1CoreOs => "l1_core_os",
            Self::L2ContextShell => "l2_context_shell",
            Self::L3PersistentUtilities => "l3_persistent_utilities",
            Self::L4UserConfig => "l4_user_config",
            Self::L4CliOverride => "l4_cli_override",
        }
    }

    /// 简略代号 ("L1", "L2", "L3", "L4", "CLI")
    pub fn code(&self) -> &'static str {
        match self {
            Self::L1CoreOs => "L1",
            Self::L2ContextShell => "L2",
            Self::L3PersistentUtilities => "L3",
            Self::L4UserConfig => "L4",
            Self::L4CliOverride => "CLI",
        }
    }

    /// 获取指定语言的本地化显示标签
    pub fn localized_label(&self, lang: Language) -> &'static str {
        match self {
            Self::L1CoreOs => match lang {
                Language::ZhHans => "L1:系统核心",
                Language::ZhHant => "L1:系統核心",
                Language::Ja => "L1:システムコア",
                Language::Ko => "L1:시스템 코어",
                Language::Fr => "L1:Cœur système",
                Language::De => "L1:Systemkern",
                Language::Es => "L1:Núcleo del sistema",
                Language::Pt => "L1:Núcleo do sistema",
                Language::It => "L1:Core di sistema",
                Language::Ru => "L1:Ядро системы",
                Language::Nl => "L1:Systeemkern",
                Language::Pl => "L1:Rdzeń systemu",
                Language::Tr => "L1:Sistem Çekirdeği",
                Language::Ar => "L1:نواة النظام",
                Language::Th => "L1:แกนระบบ",
                Language::Vi => "L1:Cốt lõi hệ thống",
                Language::Id => "L1:Inti Sistem",
                Language::Sv => "L1:Systemkärna",
                Language::Da => "L1:Systemkerne",
                Language::Nb => "L1:Systemkjerne",
                Language::Fi => "L1:Järjestelmäydin",
                Language::Cs => "L1:Jádro systému",
                Language::Uk => "L1:Ядро системи",
                Language::En => "L1: Core OS",
            },
            Self::L2ContextShell => match lang {
                Language::ZhHans => "L2:会话终端",
                Language::ZhHant => "L2:工作階段終端",
                Language::Ja => "L2:セッション端末",
                Language::Ko => "L2:세션 터미널",
                Language::Fr => "L2:Terminal de session",
                Language::De => "L2:Sitzungs-Shell",
                Language::Es => "L2:Terminal de sesión",
                Language::Pt => "L2:Terminal de sessão",
                Language::It => "L2:Terminale di sessione",
                Language::Ru => "L2:Терминал сессии",
                Language::Nl => "L2:Context-shell",
                Language::Pl => "L2:Powłoka sesji",
                Language::Tr => "L2:Oturum Kabuğu",
                Language::Ar => "L2:صدفة الجلسة",
                Language::Th => "L2:เชลล์เซสชัน",
                Language::Vi => "L2:Phiên làm việc",
                Language::Id => "L2:Shell Konteks",
                Language::Sv => "L2:Kontextskal",
                Language::Da => "L2:Kontext-shell",
                Language::Nb => "L2:Kontekst-skall",
                Language::Fi => "L2:Kontekstikuori",
                Language::Cs => "L2:Kontextový shell",
                Language::Uk => "L2:Термінал сесії",
                Language::En => "L2: Context Shell",
            },
            Self::L3PersistentUtilities => match lang {
                Language::ZhHans => "L3:常驻设施",
                Language::ZhHant => "L3:常駐設施",
                Language::Ja => "L3:常駐ユーティリティ",
                Language::Ko => "L3:상주 유틸리티",
                Language::Fr => "L3:Utilitaires résidents",
                Language::De => "L3:Dienstprogramme",
                Language::Es => "L3:Utilidades residentes",
                Language::Pt => "L3:Utilitários residentes",
                Language::It => "L3:Utilità residenti",
                Language::Ru => "L3:Системные утилиты",
                Language::Nl => "L3:Permanente tools",
                Language::Pl => "L3:Narzędzia rezydentne",
                Language::Tr => "L3:Yerleşik Araçlar",
                Language::Ar => "L3:الأدوات الدائمة",
                Language::Th => "L3:ยูทิลิตี้ประจำ",
                Language::Vi => "L3:Tiện ích thường trú",
                Language::Id => "L3:Utilitas Residen",
                Language::Sv => "L3:Residenta verktyg",
                Language::Da => "L3:Permanente hjælpeprogrammer",
                Language::Nb => "L3:Faste verktøy",
                Language::Fi => "L3:Pysyvät apuohjelmat",
                Language::Cs => "L3:Rezidentní nástroje",
                Language::Uk => "L3:Системні утиліти",
                Language::En => "L3: Persistent Utilities",
            },
            Self::L4UserConfig => match lang {
                Language::ZhHans => "L4:用户配置",
                Language::ZhHant => "L4:使用者設定",
                Language::Ja => "L4:ユーザー設定",
                Language::Ko => "L4:사용자 설정",
                Language::Fr => "L4:Config utilisateur",
                Language::De => "L4:Benutzerkonfig",
                Language::Es => "L4:Config. de usuario",
                Language::Pt => "L4:Config. do usuário",
                Language::It => "L4:Config. utente",
                Language::Ru => "L4:Пользовательская",
                Language::Nl => "L4:Gebruikersconfig",
                Language::Pl => "L4:Konfiguracja użytkownika",
                Language::Tr => "L4:Kullanıcı Yapılandırması",
                Language::Ar => "L4:تكوين المستخدم",
                Language::Th => "L4:การกำหนดค่าผู้ใช้",
                Language::Vi => "L4:Cấu hình người dùng",
                Language::Id => "L4:Konfigurasi Pengguna",
                Language::Sv => "L4:Användarkonfig",
                Language::Da => "L4:Brugerkonfiguration",
                Language::Nb => "L4:Brukerkonfig",
                Language::Fi => "L4:Käyttäjäasetukset",
                Language::Cs => "L4:Uživatelská konfigurace",
                Language::Uk => "L4:Конфігурація користувача",
                Language::En => "L4: User Config",
            },
            Self::L4CliOverride => match lang {
                Language::ZhHans => "L4:CLI保留",
                Language::ZhHant => "L4:CLI覆寫",
                Language::Ja => "L4:CLIオーバーライド",
                Language::Ko => "L4:CLI 재정의",
                Language::Fr => "L4:Surcharge CLI",
                Language::De => "L4:CLI-Überschreibung",
                Language::Es => "L4:Anulación de CLI",
                Language::Pt => "L4:Substituição de CLI",
                Language::It => "L4:Sovrascrittura CLI",
                Language::Ru => "L4:Параметр CLI",
                Language::Nl => "L4:CLI-overschrijving",
                Language::Pl => "L4:Nadpisanie CLI",
                Language::Tr => "L4:CLI Geçersiz Kılma",
                Language::Ar => "L4:تجاوز CLI",
                Language::Th => "L4:การแทนที่ CLI",
                Language::Vi => "L4:Ghi đè CLI",
                Language::Id => "L4:Penggantian CLI",
                Language::Sv => "L4:CLI-undantag",
                Language::Da => "L4:CLI-tilsidesættelse",
                Language::Nb => "L4:CLI-overstyring",
                Language::Fi => "L4:CLI-ohitus",
                Language::Cs => "L4:Přepsání CLI",
                Language::Uk => "L4:Перевизначення CLI",
                Language::En => "L4: CLI Override",
            },
        }
    }
}

impl TerminationStatusCode {
    /// 状态码对应的国际化短文本
    pub fn localized_description(&self, lang: Language) -> &'static str {
        match self {
            Self::SuccessSigterm => match lang {
                Language::ZhHans => "标准终止成功 (SIGTERM)",
                Language::ZhHant => "標準結束成功 (SIGTERM)",
                Language::Ja => "正常終了 (SIGTERM)",
                _ => "Terminated gracefully (SIGTERM)",
            },
            Self::SuccessSigkill => match lang {
                Language::ZhHans => "强制终止成功 (SIGKILL)",
                Language::ZhHant => "強制結束成功 (SIGKILL)",
                Language::Ja => "強制終了 (SIGKILL)",
                _ => "Force terminated (SIGKILL)",
            },
            Self::SuccessAppKit => match lang {
                Language::ZhHans => "终止成功 (AppKit)",
                Language::ZhHant => "結束成功 (AppKit)",
                Language::Ja => "終了成功 (AppKit)",
                _ => "Terminated via AppKit",
            },
            Self::SkippedCallerLineage => match lang {
                Language::ZhHans => "保护调用者会话链路，跳过终止",
                Language::ZhHant => "保護呼叫者工作階段連結，跳過結束",
                Language::Ja => "呼び出し元セッション保護のためスキップ",
                _ => "Caller session protected, skipped",
            },
            Self::SkippedCriticalDaemon => match lang {
                Language::ZhHans => "系统底层守护进程受常驻保护，已跳过终止",
                Language::ZhHant => "系統底層守護程序受常駐保護，已跳過結束",
                Language::Ja => "システム保護プロセスのためスキップ",
                _ => "Critical system daemon protected, skipped",
            },
            Self::FailedPermissionDenied => match lang {
                Language::ZhHans => "权限拒绝",
                Language::ZhHant => "權限被拒絕",
                Language::Ja => "アクセス拒否",
                _ => "Permission denied (EPERM)",
            },
            Self::FailedDispatch => match lang {
                Language::ZhHans => "信号派发失败",
                Language::ZhHant => "訊號派發失敗",
                Language::Ja => "シグナル送信失敗",
                _ => "Signal dispatch failed",
            },
            Self::FailedTimeout => match lang {
                Language::ZhHans => "终止超时 (访达)",
                Language::ZhHant => "結束逾時 (Finder)",
                Language::Ja => "終了タイムアウト (Finder)",
                _ => "Termination timed out (Finder)",
            },
            Self::FailedKill => match lang {
                Language::ZhHans => "强制终止失败 (SIGKILL)",
                Language::ZhHant => "強制結束失敗 (SIGKILL)",
                Language::Ja => "強制終了失敗 (SIGKILL)",
                _ => "Force termination failed (SIGKILL)",
            },
            Self::Unknown => match lang {
                Language::ZhHans => "未知状态",
                Language::ZhHant => "未知狀態",
                Language::Ja => "不明な状態",
                _ => "Unknown status",
            },
        }
    }
}

/// 国际化错误信息格式化辅助工具
pub struct CoreMessages;

impl CoreMessages {
    pub fn caller_lineage_error(pid: i32, lang: Language) -> String {
        match lang {
            Language::ZhHans => format!("进程 PID {} 属于当前调用者祖先会话链路", pid),
            Language::ZhHant => format!("處理程序 PID {} 屬於目前呼叫者工作階段連結", pid),
            Language::Ja => format!("プロセス PID {} は現在の呼び出し元セッションに属しています", pid),
            _ => format!("Process PID {} belongs to the caller session lineage", pid),
        }
    }

    pub fn critical_daemon_error(lang: Language) -> &'static str {
        match lang {
            Language::ZhHans => "系统底层关键守护服务禁止强制终止",
            Language::ZhHant => "系統底層關鍵守護服務禁止強制結束",
            Language::Ja => "重要なシステムデーモンは終了できません",
            _ => "Critical system daemons cannot be forcibly terminated",
        }
    }

    pub fn finder_timeout_error(lang: Language) -> &'static str {
        match lang {
            Language::ZhHans => {
                "访达未能按时响应退出请求，避免发送 SIGKILL 导致 launchd 异常闪回重拉"
            }
            Language::ZhHant => {
                "Finder 未能即時回應結束請求，避免傳送 SIGKILL 導致 launchd 異常閃回重新啟動"
            }
            Language::Ja => {
                "Finder が制限時間内に終了しなかったため、launchd による異常再起動を防ぐべく SIGKILL 送信を回避しました"
            }
            _ => {
                "Finder did not respond in time; avoiding SIGKILL to prevent launchd abnormal respawn"
            }
        }
    }

    pub fn purge_failed_error(code: Option<i32>, detail: &str, lang: Language) -> String {
        match lang {
            Language::ZhHans => format!("purge 执行失败 (退出码 {:?}): {}", code, detail),
            Language::ZhHant => format!("purge 執行失敗 (結束代碼 {:?}): {}", code, detail),
            Language::Ja => format!("purge の実行に失敗しました (終了コード {:?}): {}", code, detail),
            _ => format!("purge failed (exit code {:?}): {}", code, detail),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_locale_str() {
        assert_eq!(Language::from_locale_str("en-US"), Language::En);
        assert_eq!(Language::from_locale_str("zh-Hans-CN"), Language::ZhHans);
        assert_eq!(Language::from_locale_str("zh-TW"), Language::ZhHant);
        assert_eq!(Language::from_locale_str("ja_JP"), Language::Ja);
        assert_eq!(Language::from_locale_str("fr-FR"), Language::Fr);
        assert_eq!(Language::from_locale_str("de-DE"), Language::De);
        assert_eq!(Language::from_locale_str("uk_UA"), Language::Uk);
    }

    #[test]
    fn test_whitelist_tier_localization() {
        let t1 = WhitelistTier::L1CoreOs;
        assert_eq!(t1.code(), "L1");
        assert_eq!(t1.identifier(), "l1_core_os");
        assert_eq!(t1.localized_label(Language::En), "L1: Core OS");
        assert_eq!(t1.localized_label(Language::ZhHans), "L1:系统核心");
        assert_eq!(t1.localized_label(Language::ZhHant), "L1:系統核心");
        assert_eq!(t1.localized_label(Language::Ja), "L1:システムコア");

        let t2 = WhitelistTier::L2ContextShell;
        assert_eq!(t2.localized_label(Language::En), "L2: Context Shell");
        assert_eq!(t2.localized_label(Language::ZhHans), "L2:会话终端");

        let t4 = WhitelistTier::L4CliOverride;
        assert_eq!(t4.code(), "CLI");
        assert_eq!(t4.identifier(), "l4_cli_override");
        assert_eq!(t4.localized_label(Language::En), "L4: CLI Override");
    }

    #[test]
    fn test_status_code_localization() {
        let sc = TerminationStatusCode::SuccessSigterm;
        assert_eq!(
            sc.localized_description(Language::En),
            "Terminated gracefully (SIGTERM)"
        );
        assert_eq!(
            sc.localized_description(Language::ZhHans),
            "标准终止成功 (SIGTERM)"
        );

        let sc_daemon = TerminationStatusCode::SkippedCriticalDaemon;
        assert_eq!(
            sc_daemon.localized_description(Language::En),
            "Critical system daemon protected, skipped"
        );
    }

    #[test]
    fn test_detect_system_language() {
        let lang = Language::detect_system();
        println!("Detected system language in Core: {:?}", lang);
    }
}
