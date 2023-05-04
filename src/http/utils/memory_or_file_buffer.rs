use std::path::PathBuf;

use actix_web::Error;
use bytes::{Bytes, BytesMut};
use data_encoding::HEXLOWER;
use futures::TryStreamExt;
use futures_core::Stream;
use sha2::{digest::DynDigest, Digest, Sha256};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio_util::io::ReaderStream;

const MAX_IN_MEMORY_FILE_SIZE: usize = 10 * 1024 * 1024;

pub struct MemoryOrFileBuffer {
    file: Option<File>,
    buf: BytesMut,
    filepath: PathBuf,
    sha256_hasher: Box<dyn DynDigest>,
    output_len: u64,
}

impl MemoryOrFileBuffer {
    pub fn new(filepath: PathBuf) -> MemoryOrFileBuffer {
        MemoryOrFileBuffer {
            file: None,
            buf: BytesMut::new(),
            filepath,
            sha256_hasher: Box::new(Sha256::new()),
            output_len: 0,
        }
    }

    pub async fn append(&mut self, bytes: Bytes) {
        self.output_len += bytes.len() as u64;
        self.sha256_hasher.update(&bytes);

        match &mut self.file {
            None => {
                if self.buf.len() < MAX_IN_MEMORY_FILE_SIZE {
                    log::info!("going memory");
                    self.buf.extend(bytes);
                } else {
                    log::info!("going file");
                    let mut f = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .open(&self.filepath)
                        .await
                        .unwrap();

                    f.write_all(&self.buf).await.unwrap();
                    f.write_all(&bytes).await.unwrap();
                    self.file = Some(f);
                }
            }
            Some(f) => {
                f.write_all(&bytes).await.unwrap();
            }
        }
    }

    pub async fn to_stream(
        &mut self,
    ) -> Box<dyn Stream<Item = Result<bytes::Bytes, Error>> + Unpin> {
        match &mut self.file {
            Some(f) => {
                let mut f2 = f.try_clone().await.unwrap();
                f2.rewind().await.unwrap();
                let buf = tokio::io::BufReader::new(f2);
                let stream = ReaderStream::new(buf).map_err(Error::from);
                Box::new(stream)
            }
            None => {
                let cloned = self.buf.clone();
                Box::new(Box::pin(futures::stream::once(async {
                    Ok(cloned.freeze())
                })))
            }
        }
    }

    pub fn sha256_and_len(&self) -> (String, u64) {
        let hash = self.sha256_hasher.clone().finalize();
        let sha256 = HEXLOWER.encode(&hash);
        (sha256, self.output_len)
    }
}

impl Drop for MemoryOrFileBuffer {
    fn drop(&mut self) {
        if self.file.is_some() {
            std::fs::remove_file(&self.filepath)
                .unwrap_or_else(|_| panic!("unable to remove file {}", &self.filepath.display()));
        }
    }
}
