use console_subscriber::ConsoleLayer;
use std::error::Error;
use time::macros::offset;
use tracing::{self, instrument};
use tracing_rolling::{Checker, Daily};
use tracing_subscriber::{self as ts, prelude::*, EnvFilter};

pub(crate) async fn setup() -> Result<(), Box<dyn Error>> {
    // TODO:
    // Replace this with a getter/setter based on whether
    // this is run from docker
    let log_dir = std::env::var("LOGS_NEW_DEV").unwrap();

    let console_layer = ConsoleLayer::builder().with_default_env().spawn();

    let console_fmt_layer = ts::fmt::layer()
        .with_ansi(atty::is(atty::Stream::Stdout))
        .with_target(true);

    /*
    let log_fmt_layer = ts::fmt::layer()
        .with_ansi(false)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .with_writer(|| {
            Daily::new(log_dir, "[year][month][day]", offset!(+8))
                .buffered()
                .build()
                .unwrap()
        });
    */
    let filter_layer =
        EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("debug"))?;

    let registry = ts::registry()
        // .with(log_fmt_layer)
        .with(console_layer)
        .with(console_fmt_layer)
        .with(filter_layer)
        .init();

    Ok(())
}
