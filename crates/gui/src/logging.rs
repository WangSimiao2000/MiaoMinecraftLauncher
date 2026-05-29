use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::prelude::*;

pub fn init(logs_dir: &Path) -> Option<WorkerGuard> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,wgpu_core=warn,wgpu_hal=warn,naga=warn"));

    let (file_writer, guard) = match std::fs::create_dir_all(logs_dir) {
        Ok(()) => {
            let appender = RollingFileAppender::new(Rotation::DAILY, logs_dir, "mmcl.log");
            let (nb, guard) = tracing_appender::non_blocking(appender);
            (Some(nb), Some(guard))
        }
        Err(_) => (None, None),
    };

    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr.with_max_level(tracing::Level::INFO))
        .with_target(false);

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(stderr_layer);

    if let Some(nb) = file_writer {
        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(nb)
            .with_ansi(false)
            .with_target(true);
        registry.with(file_layer).init();
    } else {
        registry.init();
    }

    install_panic_hook();
    guard
}

fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("<non-string panic payload>");
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown>".to_string());
        tracing::error!(target: "miao_gui::panic", location = %location, payload = %payload, "panic");
        default_hook(info);
    }));
}
