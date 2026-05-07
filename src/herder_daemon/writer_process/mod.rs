//! This module has logic for the child process that writes to the disk.
//!
//! IT IS NOT TO BE USED DIRECTLY BY THE USER! ITS API HAS NO STABILITY
//! GUARANTEES!

use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek},
    os::unix::process::ExitStatusExt,
    process::{Command, Stdio},
    thread::JoinHandle,
};

use tracing::{debug, info};
use tracing_unwrap::ResultExt;

use self::{
    xplat::open_blockdev,
};
use crate::{ device, herder_api::write_verify::*, legacy_io::{SyncDataFile, VerifyOp, WriteOp}};

mod xplat;

/// Maximum size we may allocate for each buffer.
const MAX_BUF_SIZE: usize = 1 << 20; // 1MiB

/// How many bytes should be written before we perform a checkpoint (aka report
/// progress).
const CHECKPOINT_BYTES: usize = 8 * (1 << 20); // 8MiB

pub fn spawn_writer(
    id: u64,
    mut tx: impl FnMut(WriteVerifyEvent) + Send + 'static,
    init_config: WriteVerifyAction,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name(format!("writer/{id}"))
        .spawn(move || {
            debug!("Spawned child thread {:?}", std::thread::current().id());

            let final_msg = match run(&mut tx, &init_config) {
                Ok(_) => WriteVerifyEvent::Success,
                Err(e) => WriteVerifyEvent::Error(e),
            };

            info!(?final_msg, "Completed");
            tx(final_msg);
        })
        .unwrap()
}

fn run(
    mut tx: impl FnMut(WriteVerifyEvent),
    args: &WriteVerifyAction,
) -> Result<(), LegacyWriteVerifyError> {
    if cfg!(target_os = "macos") && args.target_type == device::Type::Disk {
        let mut command = Command::new("diskutil");
        command
            .arg("unmountdisk")
            .arg(&args.dest)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        info!(?command, "spawning process to unmount disk");
        let mut child = command.spawn()?;
        debug!("successfully ran diskutil, waiting on child process");

        let exit = child.wait()?;
        let mut stderr = String::new();
        child.stderr.take().unwrap().read_to_string(&mut stderr)?;
        let mut stdout = String::new();
        child.stdout.take().unwrap().read_to_string(&mut stdout)?;

        debug!(?exit, ?stderr, "child exited");

        let exit_code = exit.into_raw();
        if !exit.success() {
            return Err(LegacyWriteVerifyError::FailedToUnmount {
                message: format!("stderr: {stderr}\nstdout: {stdout}"),
                exit_code,
            });
        }
    }

    info!("Opening file {}", args.src.to_string_lossy());
    let mut file = File::open(&args.src).unwrap_or_log();
    let size = file.seek(io::SeekFrom::End(0))?;
    file.seek(io::SeekFrom::Start(0))?;

    info!(size, "Got input file size");

    info!("Opening {} for writing", args.dest.to_string_lossy());

    let mut disk = SyncDataFile(match args.target_type {
        device::Type::File => OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&args.dest)?,
        device::Type::Disk | device::Type::Partition => {
            open_blockdev(&args.dest, args.compression)?
        }
    });

    tx(WriteVerifyEvent::InitSuccess(WriteVerifyStart {
        input_file_bytes: size,
    }));

    let bs = match args.block_size {
        Some(bs) => bs,
        None => {
            info!("Unknown block size, assuming 512");
            512
        }
    };
    let buf_size = ((bs * 2048) as usize).min(MAX_BUF_SIZE);
    let checkpoint_period = CHECKPOINT_BYTES / buf_size;

    let actual_input_bytes = WriteOp {
        file: &mut file,
        disk: &mut disk,
        cf: args.compression,
        buf_size,
        disk_block_size: bs as usize,
        checkpoint_period,
        file_read_buf_size: buf_size,
    }
    .execute(&mut tx)?;

    tx(WriteVerifyEvent::FinishedWriting {
        verifying: args.verify,
    });

    if !args.verify {
        info!("Verification skip was requested, stopping");
        return Ok(());
    }

    info!("Rewinding source and target to beginning");
    file.seek(io::SeekFrom::Start(0))?;
    disk.seek(io::SeekFrom::Start(0))?;

    if args.target_type == device::Type::File {
        info!(
            ?actual_input_bytes,
            "Output is a file, truncating to input length in case we wrote too much"
        );
        disk.0.set_len(actual_input_bytes)?;
    };

    info!("Executing verification");
    VerifyOp {
        file: &mut file,
        disk: &mut disk,
        cf: args.compression,
        buf_size,
        disk_block_size: bs as usize,
        checkpoint_period,
        file_read_buf_size: buf_size,
    }
    .execute(tx)?;

    Ok(())
}
