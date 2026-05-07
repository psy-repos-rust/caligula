use std::io::{BufReader, Read, Seek};

use anyhow::Context as _;
use bytesize::ByteSize;

use crate::{
    compression::CompressionFormat,
    hash::{FileHashInfo, HashAlg, Hashing},
};

pub fn do_file_hashing(
    file: impl Read + Seek,
    cf: CompressionFormat,
    alg: HashAlg,
    mut checkpoint: impl FnMut(u64),
) -> anyhow::Result<FileHashInfo> {
    let decompress = crate::compression::decompress(cf, BufReader::new(file))
        .context("Failed to open input file with decompressor")?;

    let mut hashing = Hashing::new(
        alg,
        decompress,
        ByteSize::kib(512).as_u64() as usize, // TODO
    );
    loop {
        for _ in 0..32 {
            match hashing.next() {
                Some(_) => {}
                None => return Ok(hashing.finalize()?),
            }
        }
        checkpoint(hashing.get_reader_mut().get_mut().stream_position()?);
    }
}
