pub mod config;
pub mod decoder;
pub mod encoder;
pub mod file;
pub mod proxy;

const HEADER_DS_PROXY : &[u8] = b"J'apercus l'audacieux capitaine.";
const HEADER_DS_PROXY_SIZE : usize = 32;
const HEADER_DS_VERSION_NB: u32 = 1;
const HEADER_DS_VERSION_NB_SIZE: usize = 4;
