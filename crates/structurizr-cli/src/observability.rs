use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

use anyhow::{Context, Result, anyhow};
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, writer::BoxMakeWriter},
    prelude::*,
};

const LOG_FORMAT_ENV: &str = "STRZ_LOG_FORMAT";
const LOG_FILE_ENV: &str = "STRZ_LOG_FILE";

static OBSERVABILITY_INITIALIZED: OnceLock<()> = OnceLock::new();

#[derive(Clone)]
struct SharedFileWriter {
    file: Arc<Mutex<File>>,
}

impl Write for SharedFileWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.file
            .lock()
            .map_err(|_| io::Error::other("observability log file lock should not be poisoned"))?
            .write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file
            .lock()
            .map_err(|_| io::Error::other("observability log file lock should not be poisoned"))?
            .flush()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogFormat {
    Compact,
    Json,
}

impl LogFormat {
    fn from_env() -> Result<Self> {
        match env::var(LOG_FORMAT_ENV) {
            Ok(value) if value.eq_ignore_ascii_case("compact") => Ok(Self::Compact),
            Ok(value) if value.eq_ignore_ascii_case("json") => Ok(Self::Json),
            Ok(value) => Err(anyhow!(
                "unsupported {LOG_FORMAT_ENV} value `{value}`; expected `compact` or `json`"
            )),
            Err(env::VarError::NotPresent) => Ok(Self::Compact),
            Err(env::VarError::NotUnicode(_)) => {
                Err(anyhow!("{LOG_FORMAT_ENV} should contain valid UTF-8"))
            }
        }
    }
}

struct OutputWriter {
    make_writer: BoxMakeWriter,
    supports_ansi: bool,
}

pub fn init_from_env() -> Result<()> {
    if OBSERVABILITY_INITIALIZED.get().is_some() || !observability_requested() {
        return Ok(());
    }

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let format = LogFormat::from_env()?;
    let output = output_writer_from_env()?;

    install_subscriber(filter, format, output)
        .context("while attempting to install the tracing subscriber")?;

    let _ = OBSERVABILITY_INITIALIZED.set(());
    Ok(())
}

fn observability_requested() -> bool {
    env::var_os(EnvFilter::DEFAULT_ENV).is_some()
        || env::var_os(LOG_FORMAT_ENV).is_some()
        || env::var_os(LOG_FILE_ENV).is_some()
}

fn output_writer_from_env() -> Result<OutputWriter> {
    let Some(path) = env::var_os(LOG_FILE_ENV) else {
        return Ok(OutputWriter {
            make_writer: BoxMakeWriter::new(io::stderr),
            supports_ansi: true,
        });
    };

    let path = PathBuf::from(path);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "while attempting to create observability log directory `{}`",
                parent.display()
            )
        })?;
    }

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .with_context(|| {
            format!(
                "while attempting to open observability log file `{}`",
                path.display()
            )
        })?;
    let shared_writer = SharedFileWriter {
        file: Arc::new(Mutex::new(file)),
    };

    Ok(OutputWriter {
        make_writer: BoxMakeWriter::new(move || shared_writer.clone()),
        supports_ansi: false,
    })
}

fn install_subscriber(filter: EnvFilter, format: LogFormat, output: OutputWriter) -> Result<()> {
    let OutputWriter {
        make_writer,
        supports_ansi,
    } = output;
    let base_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_ansi(supports_ansi)
        .with_writer(make_writer);

    match format {
        LogFormat::Compact => tracing_subscriber::registry()
            .with(filter)
            .with(base_layer.compact())
            .try_init()
            .map_err(Into::into),
        LogFormat::Json => tracing_subscriber::registry()
            .with(filter)
            .with(base_layer.json())
            .try_init()
            .map_err(Into::into),
    }
}
