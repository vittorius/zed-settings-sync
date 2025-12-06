use std::env;
use tracing::Level;
use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logger() {
    let log_level = env::var("ZED_SETTINGS_SYNC_LOG_LEVEL")
        .unwrap_or_else(|_| "info".to_string())
        .to_lowercase();

    let level = match log_level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => unreachable!(),
    };

    #[allow(clippy::expect_used)]
    let filter = EnvFilter::from_default_env()
        .add_directive(
            format!("zed_settings_sync={level}")
                .parse()
                .expect("Failed to parse log filter directive"),
        )
        .add_directive(
            "tower_lsp=off"
                .parse()
                .expect("Failed to parse log filter directive"),
        ); // silence tower-lsp

    let stderr_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_filter(filter);

    tracing_subscriber::registry().with(stderr_layer).init();
}
