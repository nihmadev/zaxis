//! A minimal logger for hosts that have none, which is the case for every Vulkan layer: the
//! process is not Rust code. Nothing is ever written to stdout, and nothing at all unless the
//! environment asks: `ZAXIS_HOOK_LOG=stderr`, or a file path to append to; `ZAXIS_HOOK_LOG_LEVEL`
//! is `error`, `warn` (default), `info`, `debug` or `trace`.

use log::{LevelFilter, Log, Metadata, Record};
use std::{fs::File, io::Write, sync::Mutex};

struct EnvLogger {
    file: Option<Mutex<File>>,
    level: LevelFilter,
}

impl Log for EnvLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level && metadata.target().starts_with("z_hook")
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!("[z-hook {}] {}\n", record.level(), record.args());
        match &self.file {
            Some(file) => {
                if let Ok(mut file) = file.lock() {
                    let _ = file.write_all(line.as_bytes());
                }
            }
            None => {
                let _ = std::io::stderr().write_all(line.as_bytes());
            }
        }
    }

    fn flush(&self) {}
}

/// Install the environment-driven logger if nothing logs yet and the environment asks for it.
/// Returns whether it was installed. A host that already set a `log` logger keeps it.
pub fn init_from_env() -> bool {
    let Ok(target) = std::env::var("ZAXIS_HOOK_LOG") else {
        return false;
    };
    let file = (target != "stderr")
        .then(|| {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&target)
                .ok()
                .map(Mutex::new)
        })
        .flatten();
    if target != "stderr" && file.is_none() {
        return false;
    }
    let level = std::env::var("ZAXIS_HOOK_LOG_LEVEL")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(LevelFilter::Warn);
    if log::set_boxed_logger(Box::new(EnvLogger { file, level })).is_err() {
        return false;
    }
    log::set_max_level(level);
    true
}
