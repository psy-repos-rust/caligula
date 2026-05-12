use std::collections::BTreeMap;

use tracing::{info, warn};

pub(crate) fn log_program_info() {
    let name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");
    tracing::info!("Starting {name} v{version}");
}

pub(crate) fn log_uname() {
    let Ok(uname) = uname::uname().inspect_err(|e| warn!("Unable to get uname info: {e}")) else {
        return;
    };
    info!(?uname.sysname, ?uname.version, ?uname.release, ?uname.machine, "uname info");
}

/// Log the contents of some files containing useful debug info about the user's
/// system
pub(crate) fn log_info_files() {
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

pub(crate) fn log_environment_variables() {
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
