pub mod config;
pub mod decoder;
pub mod encoder;
pub mod file;
pub mod proxy;

const HEADER_DS_PROXY : &[u8] = b"j'apercus l'audacieux capitaine, cramponne a l'une des nageoires de l'animal";
const HEADER_DS_VERSION_NB: u32 = 1;
const HEADER_DS_VERSION_NB_SIZE: usize = 4;
