pub const PREFIX: &[u8] = b"J'apercus l'audacieux capitaine.";
pub const PREFIX_SIZE: usize = 32;
const VERSION_NB: usize = 2;
pub const VERSION_NB_SIZE: usize = 8;
const CHUNK_SIZE_SIZE: usize = 8; //usize size
const KEY_ID_SIZE: usize = 8; //u64 size
pub const HEADER_SIZE: usize = PREFIX_SIZE + VERSION_NB_SIZE + CHUNK_SIZE_SIZE;
pub const HEADER_V2_SIZE: usize = HEADER_SIZE + KEY_ID_SIZE;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Header {
    version: usize,
    pub chunk_size: usize,
    pub key_id: u64,
}

impl Header {
    pub fn new(chunk_size: usize, key_id: u64) -> Header {
        Header {
            version: VERSION_NB,
            chunk_size,
            key_id,
        }
    }
}

impl From<Header> for Vec<u8> {
    fn from(header: Header) -> Vec<u8> {
        [
            PREFIX,
            &2_usize.to_le_bytes(),
            &header.chunk_size.to_le_bytes(),
            &header.key_id.to_le_bytes(),
        ]
        .concat()
    }
}
