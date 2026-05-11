use std::{
    fmt::{Debug, Display},
    io::Write,
};

use crossterm::{style::Stylize, terminal::disable_raw_mode};

use crate::logging::coredump::CoredumpInstructions;

/// An error augmented with a function for generating [`ErrorInfo`] out of it,
/// containing information to print in [`crash_and_burn()`] implementations.
// TODO: kill anyhow and make it `: Error` instead of `Display + Debug`
pub trait ErrorWithInfo: Display + Debug {
    /// Get the [`ErrorInfo`].
    fn error_info(&self) -> ErrorInfo;
}

impl ErrorWithInfo for anyhow::Error {
    fn error_info(&self) -> ErrorInfo {
        ErrorInfo {
            severity: ErrorSeverity::Fatal,
            remediation: RemediationAdvice::Transient,
        }
    }
}

/// Cause this program to fatally exit and print out a big error for the user to
/// read.
pub fn crash_and_burn(ctx: &ErrorContext, error: impl ErrorWithInfo) {
    tracing::error!("Crashing and burning with error! {error}");
    tracing::error!("Full debug of error: {error:#?}");
    tracing::error!("Full info of error: {:#?}", error.error_info());

    disable_raw_mode().ok();

    write_error_message_to_terminal(std::io::stderr(), ctx, error).ok();

    // abort to trigger coredump
    unsafe {
        libc::abort();
    }
}

fn write_error_message_to_terminal(
    mut w: impl Write,
    ctx: &ErrorContext,
    error: impl ErrorWithInfo,
) -> std::io::Result<()> {
    let info = error.error_info();
    let header = match info.severity {
        ErrorSeverity::Panic => "!!! PROGRAM PANICKED !!!",
        ErrorSeverity::Fatal => "Caligula encountered a fatal error and had to abort!",
        ErrorSeverity::Recoverable => {
            "Caligula could not recover from an error it should have recovered from!"
        }
    };

    writeln!(w, "{}", header.bold().red())?;
    writeln!(w, "{}", error.to_string().red())?;
    writeln!(w)?;

    match info.remediation {
        RemediationAdvice::DeveloperError => {
            writeln!(
                w,
                "{}",
                "This is most likely a bug caused by developer error."
                    .bold()
                    .yellow()
            )?;

            writeln!(
                w,
                "{} {}",
                "We would highly appreciate it if you reported it to the bug tracker:".yellow(),
                super::ISSUE_TRACKER_URL.bold().yellow()
            )?;

            writeln!(
                w,
                "{} {}",
                "Please include this log file in your bug report:".yellow(),
                ctx.log_path.clone().bold().yellow()
            )?;
            writeln!(w)?;
        }
        RemediationAdvice::Transient => {
            writeln!(
                w,
                "{} {}",
                "This is most likely a transient issue.".bold().yellow(),
                "Please try running the program again. If that doesn't work, please check your \
                 system and hardware configuration."
                    .yellow()
            )?;

            writeln!(
                w,
                "{} {}",
                "If issues still persist, we would highly appreciate it if you reported it to the \
                 bug tracker:"
                    .yellow(),
                super::ISSUE_TRACKER_URL.bold().yellow()
            )?;

            writeln!(
                w,
                "{}{}",
                "Please include this log file in your bug report: ".yellow(),
                ctx.log_path.clone().bold().yellow()
            )?;
            writeln!(w)?;
        }
    };

    write_coredump_instructions(&mut w, ctx)?;

    // add extra newlines at end to break the "core dumped" message onto its own
    // line
    write!(w, "\n\n")?;

    Ok(())
}

fn write_coredump_instructions(
    mut w: impl Write,
    ctx: &ErrorContext,
) -> Result<(), std::io::Error> {
    writeln!(
        w,
        "{}",
        "Though it's not required when reporting issues, it would help with debugging if you \
         attached a coredump in addition to your logs."
            .bold()
            .cyan(),
    )?;
    writeln!(w, "{}", ctx.coredump_instructions.to_string().cyan())?;
    Ok(())
}

/// Information to print to the user.
#[derive(Debug, PartialEq, Eq)]
pub struct ErrorInfo {
    pub severity: ErrorSeverity,
    pub remediation: RemediationAdvice,
}

pub struct ErrorContext {
    pub log_path: String,
    pub coredump_instructions: CoredumpInstructions,
}

/// How bad this error is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// It's a panic. Panics should never ever happen in this program, but they
    /// still sometimes happen, so really only the panic hook sets this value.
    Panic,

    /// It's not a panic, but it's still some kind of error we can't reasonably
    /// expect to recover from.
    Fatal,

    /// It's an error we expect to be able to recover from. If this bubbles up
    /// to the error handler, there's a chance that we didn't handle the error
    /// correctly.
    #[expect(unused)]
    Recoverable,
}

/// Advice to give the user for remediating this error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemediationAdvice {
    /// This error is entirely the developer's fault.
    DeveloperError,

    /// This error is likely transient.
    Transient,
}
