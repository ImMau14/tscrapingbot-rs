use tracing_subscriber::EnvFilter;

/// Use RUST_LOG, fallback to info if not set
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
