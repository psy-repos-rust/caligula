use std::{fs::File, path::PathBuf};

use bytesize::ByteSize;

use crate::{compression::CompressionFormat, device::WriteTarget, herder_daemon::ipc::WriteVerifyAction};


#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BeginParams {
    pub input_file: PathBuf,
    pub input_file_size: ByteSize,
    pub compression: CompressionFormat,
    pub target: WriteTarget,
}

impl BeginParams {
    pub fn new(
        input_file: PathBuf,
        compression: CompressionFormat,
        target: WriteTarget,
    ) -> std::io::Result<Self> {
        let input_file_size = ByteSize::b(File::open(&input_file)?.metadata()?.len());
        Ok(Self {
            input_file,
            input_file_size,
            compression,
            target,
        })
    }

    pub fn make_child_config(&self) -> WriteVerifyAction {
        WriteVerifyAction {
            dest: self.target.devnode.clone(),
            src: self.input_file.clone(),
            verify: true,
            compression: self.compression,
            target_type: self.target.target_type,
            block_size: self.target.block_size.0.map(|s| s.as_u64()),
        }
    }
}
