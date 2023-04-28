pub fn start_tracing(
    level: log::Level,
    filename: &str,
) -> Result<tracing_appender::non_blocking::WorkerGuard, time::error::IndeterminateOffset> {
    use time::{format_description, UtcOffset};
    use tracing_appender::rolling;
    use tracing_subscriber::{
        fmt::{self, time::OffsetTime},
        prelude::__tracing_subscriber_SubscriberExt,
        registry,
        util::SubscriberInitExt,
        EnvFilter,
    };

    let env_filter =
        EnvFilter::try_from_env("RUST_LOG").unwrap_or_else(|_| EnvFilter::new(level.as_str()));
    let formatting_layer = fmt::layer().compact().with_writer(std::io::stdout);
    let file_appender = rolling::never(std::env::temp_dir(), filename);
    let local_time = OffsetTime::new(
        UtcOffset::current_local_offset()?,
        format_description::parse(
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]",
        )
        .unwrap(),
    );
    let (non_blocking_appender, guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_timer(local_time)
        .with_writer(non_blocking_appender);
    registry()
        .with(env_filter)
        .with(formatting_layer)
        .with(file_layer)
        .init();

    Ok(guard)
}
