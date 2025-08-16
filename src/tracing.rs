use std::sync::Once;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt};

static INIT: Once = Once::new();

pub(crate) fn init() {
    INIT.call_once(|| {
        if let Ok(log_path) = std::env::var("CLX_TRACE_LOG") {
            let file_appender = tracing_appender::rolling::never(".", &log_path);
            let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

            let file_layer = fmt::layer()
                .json()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_level(true)
                .with_thread_ids(false)
                .with_thread_names(false);

            let env_filter = EnvFilter::new("clx=trace");

            let subscriber = tracing_subscriber::registry()
                .with(env_filter)
                .with(file_layer);

            let _ = tracing::subscriber::set_global_default(subscriber);

            // Keep the guard alive
            std::mem::forget(_guard);
        }
    });
}
