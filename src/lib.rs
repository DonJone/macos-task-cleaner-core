pub mod app;
pub mod signal;
pub mod whitelist;

pub use app::{scan_foreground_apps, AppTarget};
pub use signal::{
    is_process_alive, send_posix_signal, tiered_terminate, ProcessTerminationRecord,
    TerminationReport,
};
pub use whitelist::{
    GeneralConfig, TaskCleanerConfig, WhitelistManager, WhitelistMatch, WhitelistSection,
    WhitelistTier,
};
