#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::instructions;

#[cfg(target_os = "macos")]
pub fn instructions() -> CoredumpInstructions {
    // TODO: implement this later
    CoredumpInstructions::CouldNotDetermine
}

#[derive(Debug, derive_more::Display, PartialEq, Eq, Clone)]
#[cfg_attr(target_os = "linux", derive(derive_more::From))]
pub enum CoredumpInstructions {
    #[display(
        fmt = "Unfortunately, we could not determine how you retrieve coredumps on your system, \
               or if your system even performs coredumps at all."
    )]
    CouldNotDetermine,

    #[display(
        fmt = "Your system appears to write coredumps to a file like {_0} in the current working \
               directory.";
    )]
    CurrentDirectory(String),

    #[display(fmt = "Your system appears to write coredumps to files like {_0}.")]
    AbsoluteDirectory(String),

    #[cfg(target_os = "linux")]
    #[from]
    #[display(fmt = "{_0}")]
    LinuxPiped(linux::LinuxHandlerMessage),
}
