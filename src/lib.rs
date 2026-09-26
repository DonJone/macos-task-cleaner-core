pub mod app;
pub mod i18n;
pub mod signal;
pub mod whitelist;

pub use app::{scan_foreground_apps, terminate_via_appkit, AppTarget};
pub use i18n::{CoreMessages, Language};
pub use signal::{
    get_caller_lineage, get_process_parent_pid, is_finder, is_process_alive, purge_system_cache,
    send_posix_signal, terminate_with_mode, terminate_with_mode_with_lang, tiered_terminate,
    tiered_terminate_with_lang, ProcessTerminationRecord, TerminationMode, TerminationReport,
    TerminationStatusCode,
};
pub use whitelist::{
    GeneralConfig, TaskCleanerConfig, WhitelistManager, WhitelistMatch, WhitelistSection,
    WhitelistTier,
};

