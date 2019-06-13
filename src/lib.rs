pub mod config;
pub mod decoder;
pub mod encoder;
pub mod file;
pub mod proxy;

const HEADER_PREFIX: &[u8] = b"J'apercus l'audacieux capitaine.";
const HEADER_PREFIX_SIZE: usize = 32;
const HEADER_VERSION_NB: u32 = 1;
const HEADER_VERSION_NB_SIZE: usize = 4;
const HEADER_CHUNK_SIZE_SIZE: usize = 8; //usize size
const HEADER_SIZE: usize = HEADER_PREFIX_SIZE + HEADER_VERSION_NB_SIZE + HEADER_CHUNK_SIZE_SIZE;
