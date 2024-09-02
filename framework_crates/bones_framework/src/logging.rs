//! Logging module for bones. Provides implementation of global tracing subscriber and panic hook.
//!
//! Enabled with feature "logging". See docs of [`setup_logging`] for details + usage.
#![allow(clippy::needless_doctest_main)]

use std::{
    backtrace::{Backtrace, BacktraceStatus},
    error::Error,
    panic::PanicInfo,
    path::PathBuf,
};

#[allow(unused_imports)]
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    filter::{FromEnvError, ParseError},
    layer::SubscriberExt,
    EnvFilter, Layer, Registry,
};

#[allow(unused_imports)]
use tracing::{error, warn, Level};

use bones_asset::HasSchema;
use bones_lib::prelude::Deref;

/// Logging prelude
pub mod prelude {
    pub use super::{
        macros::setup_logs, setup_logging, setup_logging_default, LogFileConfig, LogFileError,
        LogFileRotation, LogPath, LogSettings,
    };
}

pub use macros::setup_logs;

/// A boxed [`Layer`] that can be used with [`setup_logging`].
pub type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync + 'static>;

/// Plugin to enable tracing. Configures global tracing subscriber.
pub struct LogSettings {
    /// Filters logs using the [`EnvFilter`] format
    pub filter: String,

    /// Filters out logs that are "less than" the given level.
    /// This can be further filtered using the `filter` setting.
    pub level: tracing::Level,

    /// Optionally add an extra [`Layer`] to the tracing subscriber
    ///
    /// This function is only called once, when logging is initialized.
    ///
    /// Because [`BoxedLayer`] takes a `dyn Layer`, `Vec<Layer>` is also an acceptable return value.
    pub custom_layer: fn() -> Option<BoxedLayer>,

    /// The (qualifier, organization, application) that will be used to pick a persistent storage
    /// location for the game.
    ///
    /// For example: `("org", "fishfolk", "jumpy")`
    ///
    /// Used to determine directory to write log files if
    // pub app_namespace: Option<(String, String, String)>,

    /// Set to write log output to file system. Not supported on wasm.
    pub log_file: Option<LogFileConfig>,
}

impl Default for LogSettings {
    fn default() -> Self {
        Self {
            filter: "wgpu=error,naga=warn".to_string(),
            level: Level::INFO,
            custom_layer: || None,
            log_file: None,
        }
    }
}

/// How often to rotate log file.
#[derive(Copy, Clone, Default)]
#[allow(missing_docs)]
pub enum LogFileRotation {
    Minutely,
    Hourly,
    #[default]
    Daily,
    Never,
}

impl From<LogFileRotation> for tracing_appender::rolling::Rotation {
    fn from(value: LogFileRotation) -> Self {
        match value {
            LogFileRotation::Minutely => Rotation::MINUTELY,
            LogFileRotation::Hourly => Rotation::HOURLY,
            LogFileRotation::Daily => Rotation::DAILY,
            LogFileRotation::Never => Rotation::NEVER,
        }
    }
}

/// Error for file logging.
#[derive(Debug, thiserror::Error)]
pub enum LogFileError {
    /// Failed to determine a log directory.
    #[error("Could not determine log dir: {0}")]
    LogDirFail(String),
    /// Attempted to setup file logging on unsupported platform.
    #[error("Logging to file system is unsupported on platform: {0}")]
    Unsupported(String),
}

/// Path to save log files. [`LogPath::find_app_data_dir`] may be used to
/// to automatically find OS appropriate app data path from app namespace strings, e.g. ("org", "fishfolk", "jumpy")
#[derive(Clone, Deref)]
pub struct LogPath(pub PathBuf);

impl LogPath {
    /// Find OS app data path for provided app namespace (e.g. ("org", "fishfolk", "jumpy"))
    ///
    /// Will error if failed to resole this directory for OS or on unsupported platform such as wasm.
    ///
    /// i.e. ~/.local/share/org.fishfolk.jumpy/logs,
    //       C:\Users\<User>\Appdata\Roaming\org.fishfolk.jumpy\logs,
    ///      ~/Library/Application Support/org.fishfolk.jumpy/logs
    #[allow(unused_variables)]
    pub fn find_app_data_dir(app_namespace: (&str, &str, &str)) -> Result<Self, LogFileError> {
        // Don't run this during tests, as Miri CI does not support the syscall.
        #[cfg(not(target_arch = "wasm32"))]
        {
            directories::ProjectDirs::from(app_namespace.0, app_namespace.1, app_namespace.2)
                // error message from `ProjectDirs::from` docs
                .ok_or(LogFileError::LogDirFail(
                    "no valid home directory path could be retrieved from the operating system"
                        .to_string(),
                ))
                .map(|dirs| LogPath(dirs.data_dir().join("logs")))
        }

        #[cfg(target_arch = "wasm32")]
        {
            Err(LogFileError::Unsupported("wasm32".to_string()))
        }
    }
}

/// Settings to enable writing tracing output to files.
pub struct LogFileConfig {
    /// Path to store log files - use [`LogPath`]'s helper function to find good default path.
    pub log_path: LogPath,

    /// How often to rotate log file.
    pub rotation: LogFileRotation,

    /// Beginning of log file name (e.g. "Jumpy.log"), timestamp will be appended to this
    /// if using rotatig logs.
    pub file_name_prefix: String,

    /// If set, will cleanup the oldest log files in directory that match `file_name_prefix` until under max
    /// file count. Otherwise no log files will be cleaned up.
    pub max_log_files: Option<usize>,
}

/// Guard for file logging thread, this should be held onto for duration of app, if dropped
/// writing to log file will stop.
///
/// It is recommended to hold onto this in main() to ensure all logs are flushed when app is
/// exiting. See [`tracing_appender::non_blocking::WorkerGuard`] docs for details.
#[derive(HasSchema)]
#[schema(no_clone, no_default)]
pub struct LogFileGuard(tracing_appender::non_blocking::WorkerGuard);

impl Drop for LogFileGuard {
    fn drop(&mut self) {
        warn!("LogFileGuard dropped - flushing buffered tracing to file, no further tracing will be written to file. If unexpected, make sure bones logging init is done in root scope of app.");
    }
}

/// Setup the global tracing subscriber, add hook for tracing panics, and optionally enable logging to file system.
///
/// This function sets panic hook to call [`tracing_panic_hook`], and then call previous hook. This writes panics to
/// tracing subscribers. This is helpful for recording panics when logging to file system.
///
/// if [`LogFileConfig`] was provided in settings and is supported on this platform (cannot log to file system on wasm),
/// this function will return a [`LogFileGuard`]. This must be kept alive for duration of process to capture all logs,
/// see [`LogFileGuard`] docs.
///
/// Examples below show direct usage and short-hand with [`setup_logs`] macro.
///
/// # Examples
///
/// ### Default without logging to file
/// ```
/// use bones_framework::logging::prelude::*;
/// fn main() {
///     let _log_guard = bones_framework::logging::setup_logging(LogSettings::default());
/// }
/// ```
/// or
/// ```
/// use bones_framework::logging::prelude::*;
/// fn main() {
///     setup_logs!();
/// }
/// ```
///
/// ### Enable logging including logging to files:
/// ```no_run
/// use bones_framework::prelude::*;
/// fn main() {
///     let log_file =
///         match LogPath::find_app_data_dir(("org", "fishfolk", "jumpy")) {
///             Ok(log_path) => Some(LogFileConfig {
///                 log_path,
///                 rotation: LogFileRotation::Daily,
///                 file_name_prefix: "Jumpy.log".to_string(),
///                 max_log_files: Some(7),
///             }),
///             Err(err) => {
///                 // Cannot use error! macro as logging not configured yet.
///                 eprintln!("Failed to configure file logging: {err}");
///                 None
///             }
///         };
///
///     // _log_guard will be dropped when main exits, remains alive for duration of program.
///     let _log_guard = bones_framework::logging::setup_logging(LogSettings {
///         log_file,
///         ..default()
///     });
/// }
/// ```
/// or logging to file with defaults:
/// ```no_run
/// use bones_framework::logging::prelude::*;
/// fn main() {
///     let _log_guard = bones_framework::logging::setup_logging_default(("org", "fishfolk", "jumpy"));
/// }
/// ```
/// same with [`macros::setup_logs`] macro:
/// ```no_run
/// use bones_framework::prelude::*;
/// fn main() {
///     setup_logs!("org", "fishfolk", "jumpy");
/// }
/// ```
///
///
#[must_use]
pub fn setup_logging(settings: LogSettings) -> Option<LogFileGuard> {
    // Preserve current panic hook, and call `tracing_panic_hook` to send panic and possibly backtrace to
    // tracing subscribers, and not just stderr.
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        tracing_panic_hook(panic_info);
        prev_hook(panic_info);
    }));

    let finished_subscriber;
    let subscriber = Registry::default();

    // add optional layer provided by user
    let subscriber = subscriber.with((settings.custom_layer)());

    let default_filter = { format!("{},{}", settings.level, settings.filter) };
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|from_env_error| {
            _ = from_env_error
                .source()
                .and_then(|source| source.downcast_ref::<ParseError>())
                .map(|parse_err| {
                    // we cannot use the `error!` macro here because the logger is not ready yet.
                    eprintln!(
                        "setup_logging() failed to parse filter from env: {}",
                        parse_err
                    );
                });

            Ok::<EnvFilter, FromEnvError>(EnvFilter::builder().parse_lossy(&default_filter))
        })
        .unwrap();
    let subscriber = subscriber.with(filter_layer);

    let log_file_guard;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let (file_layer, file_guard) = match &settings.log_file {
            Some(log_file) => {
                let LogFileConfig {
                    log_path,
                    rotation,
                    file_name_prefix,
                    max_log_files,
                } = log_file;

                let file_appender = RollingFileAppender::builder()
                    .filename_prefix(file_name_prefix)
                    .rotation((*rotation).into());

                let file_appender = match *max_log_files {
                    Some(max) => file_appender.max_log_files(max),
                    None => file_appender,
                };

                match file_appender.build(&**log_path) {
                    Ok(file_appender) => {
                        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
                        let file_layer =
                            tracing_subscriber::fmt::Layer::default().with_writer(non_blocking);
                        (Some(file_layer), Some(LogFileGuard(_guard)))
                    }
                    Err(err) => {
                        // we cannot use the `error!` macro here because the logger is not ready yet.
                        eprintln!("Failed to configure tracing_appender layer for logging to file system - {err}");
                        (None, None)
                    }
                }
            }
            None => (None, None),
        };
        let subscriber = subscriber.with(file_layer);
        log_file_guard = file_guard;

        #[cfg(feature = "tracing-tracy")]
        let tracy_layer = tracing_tracy::TracyLayer::default();

        // note: the implementation of `Default` reads from the env var NO_COLOR
        // to decide whether to use ANSI color codes, which is common convention
        // https://no-color.org/
        let fmt_layer = tracing_subscriber::fmt::Layer::default();

        // bevy_render::renderer logs a `tracy.frame_mark` event every frame
        // at Level::INFO. Formatted logs should omit it.
        #[cfg(feature = "tracing-tracy")]
        let fmt_layer = fmt_layer.with_filter(tracing_subscriber::filter::FilterFn::new(|meta| {
            meta.fields().field("tracy.frame_mark").is_none()
        }));

        let subscriber = subscriber.with(fmt_layer);

        #[cfg(feature = "tracing-tracy")]
        let subscriber = subscriber.with(tracy_layer);
        finished_subscriber = subscriber;
    }

    #[cfg(target_arch = "wasm32")]
    {
        finished_subscriber = subscriber.with(tracing_wasm::WASMLayer::new(
            tracing_wasm::WASMLayerConfig::default(),
        ));
        log_file_guard = None;
    }

    if let Err(err) = tracing::subscriber::set_global_default(finished_subscriber) {
        error!("{err} - `setup_logging` was called and configures global subscriber. Game may either setup subscriber itself, or call `setup_logging` from bones, but not both.");
    }

    #[cfg(target_arch = "wasm32")]
    {
        if settings.log_file.is_some() {
            // Report this warning after setting up tracing subscriber so it will show up on wasm.
            warn!("bones_framework::setup_logging() - `LogFileConfig` provided, however logging to file system is not supported in wasm.");
        }
    }

    log_file_guard
}

/// Helper to call [`setup_logging`] conciseably with reasonable defaults for logging to console and file system.
///
/// This uses default [`LogSettings`] with addition of enabling logging to files. If logging to file is not desired,
/// you can call `setup_logging(LogSettings::default())` instead.
///
/// See [`setup_logging`] docs for details.
#[must_use]
pub fn setup_logging_default(app_namespace: (&str, &str, &str)) -> Option<LogFileGuard> {
    let file_name_prefix = format!("{}.log", app_namespace.2);
    let log_file =
        match LogPath::find_app_data_dir((app_namespace.0, app_namespace.1, app_namespace.2)) {
            Ok(log_path) => Some(LogFileConfig {
                log_path,
                rotation: LogFileRotation::Daily,
                file_name_prefix,
                max_log_files: Some(7),
            }),
            Err(err) => {
                // Cannot use error! macro as logging not configured yet.
                eprintln!("Failed to configure file logging: {err}");
                None
            }
        };

    // _log_guard will be dropped when main exits, remains alive for duration of program.
    setup_logging(LogSettings {
        log_file,
        ..Default::default()
    })
}

/// Logging macros
#[macro_use]
pub mod macros {

    /// [`setup_logs`] is a macro for initializing logging in bones.
    ///
    /// It wraps a call to [`super::setup_logging`] (see docs for details on configuration options).
    ///
    /// Warning: There may be issues if not called in root scope of app (e.g. in `main()`).
    /// Macro expands to: `let _guard = setup_logging(...);`, if `_guard` is dropped, any logging to
    /// file system will stop (console logging unimpacted).
    ///
    /// Usage for log defaults (logging to file system included):
    /// ```no_run
    /// use bones_framework::prelude::*;
    /// setup_logs!("org", "fishfolk", "jumpy");
    /// ```
    ///
    /// Usage for log defaults (without logging to file system):
    /// ```
    /// use bones_framework::prelude::*;
    /// setup_logs!();
    /// ```
    #[macro_export]
    macro_rules! setup_logs {
        // LogSettings::default() -
        //   setup_logs!();
        () => {
            use bones_framework::logging::setup_logging;
            use bones_framework::logging::LogSettings;
            let _log_file_guard = setup_logging(LogSettings::default());
        };
        // With LogSettings identifier -
        //   let settings = LogSettings::{...};
        //   setup_logs!(settings);
        ($settings:ident) => {
            use bones_framework::logging::setup_logging;
            let _log_file_guard = setup_logging($settings);
        };
        // setup_logging_default from app namespace -
        //   setup_logs!(("org", "fishfolk", "jumpy"));
        ($app_namespace:expr) => {
            use bones_framework::logging::setup_logging_default;
            let _log_file_guard = setup_logging_default($app_namespace);
        };
        // setup_logging_default from app namespace -
        //   setup_logs!("org", "fishfolk", "jumpy");
        ($app_ns1:expr, $app_ns2:expr, $app_ns3:expr) => {
            use bones_framework::logging::setup_logging_default;
            let _log_file_guard = setup_logging_default(($app_ns1, $app_ns2, $app_ns3));
        };
    }
    pub use setup_logs;
}

/// Panic hook that sends panic payload to [`tracing::error`], and backtrace if available.
///
/// This hook is enabled in [`setup_logging`] to make sure panics are traced.
pub fn tracing_panic_hook(panic_info: &PanicInfo) {
    let payload = panic_info.payload();

    let payload = if let Some(s) = payload.downcast_ref::<&str>() {
        Some(*s)
    } else {
        payload.downcast_ref::<String>().map(|s| s.as_str())
    };

    let location = panic_info.location().map(|l| l.to_string());
    let (backtrace, note) = {
        let backtrace = Backtrace::capture();
        let note = (backtrace.status() == BacktraceStatus::Disabled)
            .then_some("run with RUST_BACKTRACE=1 environment variable to display a backtrace");
        (Some(backtrace), note)
    };

    tracing::error!(
        panic.payload = payload,
        panic.location = location,
        panic.backtrace = backtrace.map(tracing::field::display),
        panic.note = note,
        "A panic occurred",
    );
}
