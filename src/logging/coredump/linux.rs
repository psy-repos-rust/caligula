use sysctl::{Ctl, CtlValue, Sysctl as _, SysctlError};
use tracing::warn;

use crate::logging::coredump::CoredumpInstructions;

const PROC_CORE_PATTERN_PATH: &str = "/proc/sys/kernel/core_pattern";

const SYSTEMD_MESSAGE: &str = "This appears to be systemd-coredump.
    You should be able to retrieve the coredump by running:
        coredumpctl dump caligula --output=core.bin
    If that doesn't work, you can list and search for all coredumps with:
        coredumpctl list";

const APPORT_MESSAGE: &str = "This appears to be Apport, which logs coredumps to /var/crash.
    You should be able to find the coredump in there, and retrieve it by running:
        apport-unpack /var/crash/NAME_OF_THE_FILE_YOU_FOUND.crash core.bin";

const ABRT_MESSAGE: &str = "This appears to be Automatic Bug Reporting Tool (ABRT), which logs \
                            coredumps to /var/spool/abrt.
    You should be able to list available coredumps with:
        abrt-cli list";

const NULL_MESSAGE: &str = "This appears to discard coredumps, so attaching one is not necessary.";

const UNKNOWN_MESSAGE: &str = "If you would like to attach a coredump, you can search for the \
                               relevant documentation on how to retrieve coredumps from this \
                               program.";

#[derive(Debug, derive_more::Display, PartialEq, Eq, Clone)]
#[display(fmt = "Your system appears to write coredumps to a handler program: \
                 {program}\n{handler_specific}")]
pub struct LinuxHandlerMessage {
    program: String,
    handler_specific: HandlerSpecific,
}

#[derive(Debug, derive_more::Display, PartialEq, Eq, Clone, Copy)]
pub enum HandlerSpecific {
    #[display(fmt = "{SYSTEMD_MESSAGE}")]
    Systemd,
    #[display(fmt = "{APPORT_MESSAGE}")]
    Apport,
    #[display(fmt = "{ABRT_MESSAGE}")]
    Abrt,
    #[display(fmt = "{NULL_MESSAGE}")]
    Null,
    #[display(fmt = "{UNKNOWN_MESSAGE}")]
    Unknown,
}

#[derive(Debug, thiserror::Error)]
enum QuerySystemError {
    #[error("Error querying sysctl: {0}")]
    Sysctl(#[from] SysctlError),
    #[error("Sysctl value was not a string: got {0}")]
    NotAString(CtlValue),
    #[error("Failed to read core_pattern from {PROC_CORE_PATTERN_PATH}")]
    ProcReadFail(std::io::Error),
}

/// Helper for generating instructions that tell Linux users how to get
/// coredumps.
#[tracing::instrument]
pub fn instructions() -> CoredumpInstructions {
    let core_pattern_result = query_sysctl().or_else(|e| {
        warn!("{e}");
        warn!("Falling back to querying {PROC_CORE_PATTERN_PATH}");
        query_proc()
    });

    match core_pattern_result {
        Ok(pattern) => parse_core_pattern(pattern),
        Err(e) => {
            warn!("{e}");
            CoredumpInstructions::CouldNotDetermine
        }
    }
}

/// More info on syntax: https://man7.org/linux/man-pages/man5/core.5.html
fn parse_core_pattern(core_pattern: String) -> CoredumpInstructions {
    if let Some(handler) = core_pattern.strip_prefix("|") {
        // From man core(5):
        // If the first character of this file is a pipe symbol (|), then the remainder
        // of the line is interpreted as the command-line for a user-space program (or
        // script) that is to be executed.

        LinuxHandlerMessage {
            program: handler.to_string(),
            handler_specific: match handler.trim() {
                s if s.contains("systemd") => HandlerSpecific::Systemd,
                s if s.contains("apport") => HandlerSpecific::Apport,
                s if s.contains("abrt") => HandlerSpecific::Abrt,
                s if s.contains("true") || s.contains("false") => HandlerSpecific::Null,
                _ => HandlerSpecific::Unknown,
            },
        }
        .into()
    } else if core_pattern.is_empty() {
        CoredumpInstructions::CouldNotDetermine
    } else if core_pattern == "/dev/null" {
        LinuxHandlerMessage {
            program: "/dev/null".to_string(),
            handler_specific: HandlerSpecific::Null,
        }
        .into()
    } else if core_pattern.starts_with('/') {
        CoredumpInstructions::AbsoluteDirectory(core_pattern)
    } else {
        CoredumpInstructions::CurrentDirectory(core_pattern)
    }
}

fn query_sysctl() -> Result<String, QuerySystemError> {
    Ctl::new("kernel.core_pattern")?
        .value()?
        .into_string()
        .map_err(QuerySystemError::NotAString)
}

fn query_proc() -> Result<String, QuerySystemError> {
    std::fs::read_to_string(PROC_CORE_PATTERN_PATH).map_err(QuerySystemError::ProcReadFail)
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        "|/nix/store/g9i3mlx33427i6a0905fyy85yghxfjxz-systemd/lib/systemd/systemd-coredump %P %u %g %s %t %c %h %d %F",
        CoredumpInstructions::LinuxPiped(
            LinuxHandlerMessage {
                program:
                    "/nix/store/g9i3mlx33427i6a0905fyy85yghxfjxz-systemd/lib/systemd/systemd-coredump \
                    %P %u %g %s %t %c %h %d %F".into(),
                handler_specific: HandlerSpecific::Systemd,
            }
        ),
    )]
    #[case(
        "|/usr/lib/systemd/systemd-coredump %P %u %g %s %t %c %h %d %F",
        CoredumpInstructions::LinuxPiped(
            LinuxHandlerMessage {
                program: "/usr/lib/systemd/systemd-coredump %P %u %g %s %t %c %h %d %F".into(),
                handler_specific: HandlerSpecific::Systemd,
            }
        ),
    )]
    #[case(
        "|/usr/share/apport/apport %p %s %c %d %P %E",
        CoredumpInstructions::LinuxPiped(
            LinuxHandlerMessage {
                program: "/usr/share/apport/apport %p %s %c %d %P %E".into(),
                handler_specific: HandlerSpecific::Apport,
            }
        ),
    )]
    #[case(
        "|/usr/lib/abrt-addon-ccpp %s %c %p %u %g %t e %P %I %h",
        CoredumpInstructions::LinuxPiped(
            LinuxHandlerMessage {
                program: "/usr/lib/abrt-addon-ccpp %s %c %p %u %g %t e %P %I %h".into(),
                handler_specific: HandlerSpecific::Abrt,
            }
        ),
    )]
    #[case(
        "|/usr/local/bin/my-custom-handler %p %s",
        CoredumpInstructions::LinuxPiped(
            LinuxHandlerMessage {
                program: "/usr/local/bin/my-custom-handler %p %s".into(),
                handler_specific: HandlerSpecific::Unknown,
            }
        ),
    )]
    #[case(
        "/dev/null",
        CoredumpInstructions::LinuxPiped(
            LinuxHandlerMessage {
                program: "/dev/null".into(),
                handler_specific: HandlerSpecific::Null,
            }
        ),
    )]
    #[case(
        "core",
        CoredumpInstructions::CurrentDirectory("core".into()),
    )]
    #[case(
        "core.%p",
        CoredumpInstructions::CurrentDirectory("core.%p".into()),
    )]
    #[case(
        "core.%e.%p",
        CoredumpInstructions::CurrentDirectory("core.%e.%p".into()),
    )]
    #[case(
        "/var/coredumps/core.%p",
        CoredumpInstructions::AbsoluteDirectory("/var/coredumps/core.%p".into()),
    )]
    #[case(
        "/tmp/cores/core.%e.%p.%t",
        CoredumpInstructions::AbsoluteDirectory("/tmp/cores/core.%e.%p.%t".into()),
    )]
    #[case("", CoredumpInstructions::CouldNotDetermine)]
    fn parse_coredump_pattern_into_message(
        #[case] input: &str,
        #[case] expected: CoredumpInstructions,
    ) {
        let result = parse_core_pattern(input.into());
        assert_eq!(result, expected)
    }
}
