mod coredump;

use std::{
    collections::BTreeMap,
    fs::File,
    io::Write,
    panic::{PanicHookInfo, set_hook},
    path::Path,
    sync::Mutex,
};

use crossterm::{style::Stylize, terminal::disable_raw_mode};
use tracing::{Level, info, warn};
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

use crate::logging::coredump::CoredumpInstructions;

const ISSUE_TRACKER_URL: &str = "https://github.com/ifd3f/caligula/issues";

/// Helper for calculating which files to log to.
#[derive(Debug, Clone)]
pub struct LogPaths {
    log_path: String,
}

impl LogPaths {
    pub fn init(state_dir: impl AsRef<Path>) -> Self {
        Self {
            log_path: if cfg!(debug_assertions) {
                "caligula.log".into()
            } else {
                state_dir
                    .as_ref()
                    .join("caligula.log")
                    .to_str()
                    .unwrap()
                    .to_owned()
            },
        }
    }

    pub fn main(&self) -> &str {
        &self.log_path
    }

    pub fn get_bug_report_msg(&self) -> String {
        format!(
            "This is likely a bug caused by developer error. Please report the issue to https://github.com/ifd3f/caligula/issues and attach the log file in {}",
            self.log_path
        )
    }
}

#[cfg(not(debug_assertions))]
const FILE_LOG_LEVEL: Level = Level::DEBUG;

#[cfg(debug_assertions)]
const FILE_LOG_LEVEL: Level = Level::TRACE;

pub fn init_logging_parent(paths: &LogPaths) {
    let log_path = paths.main().to_owned();
    let file = File::create(&log_path).expect("Failed to create log file!");

    init_tracing_subscriber(file);

    log_program_info();
    log_uname();
    log_info_files();
    log_environment_variables();

    let core_dump_msg = self::coredump::instructions();
    info!("Guessed system coredump handler to be {core_dump_msg:?}");

    set_hook(Box::new(move |p| {
        tracing_panic::panic_hook(p);

        disable_raw_mode().ok();

        PanicMessage {
            panic: p,
            log_path: &log_path,
            coredump_instructions: &core_dump_msg,
        }
        .write_to(std::io::stderr())
        .ok();

        // abort to trigger coredump
        unsafe {
            libc::abort();
        }
    }));
}

struct PanicMessage<'a> {
    panic: &'a PanicHookInfo<'a>,
    log_path: &'a str,
    coredump_instructions: &'a CoredumpInstructions,
}

impl<'a> PanicMessage<'a> {
    fn write_to(&self, mut w: impl Write) -> std::io::Result<()> {
        writeln!(w, "{}", "!!! PROGRAM PANICKED !!!".bold().red())?;
        writeln!(w, "{}", self.panic.to_string().red())?;
        writeln!(w)?;

        writeln!(
            w,
            "{}",
            "This is likely a bug caused by developer error."
                .bold()
                .yellow()
        )?;
        writeln!(
            w,
            "{}{}",
            "We would highly appreciate it if you reported it here: ".yellow(),
            ISSUE_TRACKER_URL.bold().yellow()
        )?;
        writeln!(
            w,
            "{}{}",
            "Please include this log file in your bug report: ".yellow(),
            self.log_path.bold().yellow()
        )?;
        writeln!(w)?;

        writeln!(
            w,
            "{}",
            "Though it's not required, it would help with debugging if you attached a coredump in \
             addition to your logs."
                .bold()
                .cyan(),
        )?;
        writeln!(w, "{}", self.coredump_instructions.to_string().cyan())?;

        // add extra newlines at end to break the "core dumped" message onto its own
        // line
        write!(w, "\n\n")?;

        Ok(())
    }
}

pub fn init_logging_child(write_path: impl AsRef<Path>) {
    let file = File::options().append(true).open(write_path).unwrap();
    init_tracing_subscriber(file);
    set_hook(Box::new(tracing_panic::panic_hook));
}

fn init_tracing_subscriber(file: File) {
    tracing_subscriber::fmt()
        .compact()
        .with_ansi(false) // hide colors
        .with_writer(Mutex::new(file))
        .with_span_events(FmtSpan::FULL)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(FILE_LOG_LEVEL.into())
                .from_env_lossy(),
        )
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .init();
}

fn log_program_info() {
    let name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");
    tracing::info!("Starting {name} v{version}");
}

pub fn log_uname() {
    let Ok(uname) = uname::uname().inspect_err(|e| warn!("Unable to get uname info: {e}")) else {
        return;
    };
    info!(?uname.sysname, ?uname.version, ?uname.release, ?uname.machine, "uname info");
}

/// Log the contents of some files containing useful debug info about the user's
/// system
fn log_info_files() {
    const INFO_FILES: &[&str] = &[
        "/etc/os-release",
        "/etc/lsb-release",
        "/etc/redhat_release",
        "/etc/debian_version",
        "/System/Library/CoreServices/SystemVersion.plist",
        "/sys/class/dmi/id/product_name",
    ];

    for path in INFO_FILES {
        match std::fs::read_to_string(path) {
            Ok(contents) => info!("{path} contents:\n{}", contents.trim()),
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => info!("{path} not found, skipping"),
                _ => warn!("Failed to read {path}: {e}"),
            },
        }
    }
}

fn log_environment_variables() {
    const ENVIRONMENT_VARIABLES: &[(&str, &[&str])] = &[
        (
            "Terminal emulator",
            &[
                "TERM",
                "TERM_PROGRAM",
                "TERM_PROGRAM_VERSION",
                "VTE_VERSION",
                "GNOME_TERMINAL_SCREEN",
            ],
        ),
        (
            "Color capabilities",
            &[
                "COLORTERM",
                "FORCE_COLOR",
                "NO_COLOR",
                "CLICOLOR",
                "CLICOLOR_FORCE",
            ],
        ),
        ("Multiplexer", &["TMUX", "TMUX_PANE", "SCREEN", "STY"]),
        ("Locale", &["LANG", "LC_ALL"]),
    ];

    let vars = ENVIRONMENT_VARIABLES
        .iter()
        .map(|(k, varnames)| {
            (
                *k,
                varnames
                    .iter()
                    .map(|var| (*var, std::env::var(var).ok()))
                    .collect::<BTreeMap<&'static str, Option<String>>>(),
            )
        })
        .collect::<BTreeMap<&'static str, _>>();

    tracing::info!("Environment variables detected: {vars:#?}")
}
