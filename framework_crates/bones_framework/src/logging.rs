//! Logging module for bones, enabled with feature "logging". Configures global tracing subscriber.

use std::error::Error;

use tracing::{error, Level};
use tracing_subscriber::{
    filter::{FromEnvError, ParseError},
    layer::SubscriberExt,
    EnvFilter, Layer, Registry,
};

use bones_lib::{Game, GamePlugin};

/// A boxed [`Layer`] that can be used with [`LogPlugin`].
pub type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync + 'static>;

/// Plugin to enable tracing. Configures global tracing subscriber.
pub struct LogPlugin {
    /// Filters logs using the [`EnvFilter`] format
    pub filter: String,

    /// Filters out logs that are "less than" the given level.
    /// This can be further filtered using the `filter` setting.
    pub level: tracing::Level,

    /// Optionally add an extra [`Layer`] to the tracing subscriber
    ///
    /// This function is only called once, when the plugin is installed.
    ///
    /// Because [`BoxedLayer`] takes a `dyn Layer`, `Vec<Layer>` is also an acceptable return value.
    ///
    /// Access to [`Game`] is also provided to allow for communication between the
    /// [`Subscriber`] and the [`Game`].
    ///
    /// Please see the `examples/log_layers.rs` for a complete example.
    pub custom_layer: fn(game: &mut Game) -> Option<BoxedLayer>,
}

impl Default for LogPlugin {
    fn default() -> Self {
        Self {
            filter: "wgpu=error,naga=warn".to_string(),
            level: Level::INFO,
            custom_layer: |_| None,
        }
    }
}

impl GamePlugin for LogPlugin {
    fn install(self, game: &mut Game) {
        let finished_subscriber;
        let subscriber = Registry::default();

        // add optional layer provided by user
        let subscriber = subscriber.with((self.custom_layer)(game));

        let default_filter = { format!("{},{}", self.level, self.filter) };
        let filter_layer = EnvFilter::try_from_default_env()
            .or_else(|from_env_error| {
                _ = from_env_error
                    .source()
                    .and_then(|source| source.downcast_ref::<ParseError>())
                    .map(|parse_err| {
                        // we cannot use the `error!` macro here because the logger is not ready yet.
                        eprintln!("LogPlugin failed to parse filter from env: {}", parse_err);
                    });

                Ok::<EnvFilter, FromEnvError>(EnvFilter::builder().parse_lossy(&default_filter))
            })
            .unwrap();
        let subscriber = subscriber.with(filter_layer);

        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            #[cfg(feature = "tracing-tracy")]
            let tracy_layer = tracing_tracy::TracyLayer::default();

            // note: the implementation of `Default` reads from the env var NO_COLOR
            // to decide whether to use ANSI color codes, which is common convention
            // https://no-color.org/
            let fmt_layer = tracing_subscriber::fmt::Layer::default();

            // bevy_render::renderer logs a `tracy.frame_mark` event every frame
            // at Level::INFO. Formatted logs should omit it.
            #[cfg(feature = "tracing-tracy")]
            let fmt_layer =
                fmt_layer.with_filter(tracing_subscriber::filter::FilterFn::new(|meta| {
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
        }

        if let Err(err) = tracing::subscriber::set_global_default(finished_subscriber) {
            error!("{err} - bones logging failed to set global subscriber. This was enabled with 'logging' feature flag.");
        }
    }
}
