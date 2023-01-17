use std::convert::TryFrom;
use std::convert::TryInto;

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

#[derive(Debug, PartialEq, Eq)]
pub enum HeaderParsingError {
    WrongSize,
    WrongPrefix,
    WrongVersion,
}

impl TryFrom<&[u8]> for Header {
    type Error = HeaderParsingError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        if slice.len() != HEADER_SIZE {
            return Err(HeaderParsingError::WrongSize);
        }

        if &slice[..PREFIX_SIZE] != PREFIX {
            return Err(HeaderParsingError::WrongPrefix);
        }

        if usize::from_le_bytes(
            slice[PREFIX_SIZE..PREFIX_SIZE + VERSION_NB_SIZE]
                .try_into()
                .unwrap(),
        ) != VERSION_NB
        {
            return Err(HeaderParsingError::WrongVersion);
        }

        let chunk_size = usize::from_le_bytes(
            slice[PREFIX_SIZE + VERSION_NB_SIZE..HEADER_SIZE]
                .try_into()
                .unwrap(),
        );

        Ok(Header::new(chunk_size, 0))
    }
}

impl From<Header> for Vec<u8> {
    fn from(header: Header) -> Vec<u8> {
        [
            PREFIX,
            &VERSION_NB.to_le_bytes(),
            &header.chunk_size.to_le_bytes(),
        ]
        .concat()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_wrong_size() {
        let too_small: &[u8] = &[0; HEADER_SIZE - 1];
        assert_eq!(
            Err(HeaderParsingError::WrongSize),
            Header::try_from(too_small)
        );

        let too_big: &[u8] = &[0; HEADER_SIZE + 1];
        assert_eq!(
            Err(HeaderParsingError::WrongSize),
            Header::try_from(too_big)
        );
    }

    #[test]
    fn test_deserialize_wrong_prefix() {
        let wrong_prefix: &[u8] = &[
            b"J'apercus le mechant  capitaine." as &[u8],
            &VERSION_NB.to_le_bytes(),
            &10usize.to_le_bytes(),
        ]
        .concat()[..];

        assert_eq!(
            Err(HeaderParsingError::WrongPrefix),
            Header::try_from(wrong_prefix)
        );
    }

    #[test]
    fn test_deserialize_wrong_version() {
        let wrong_version: &[u8] = &[
            PREFIX as &[u8],
            &666usize.to_le_bytes(),
            &10usize.to_le_bytes(),
        ]
        .concat()[..];

        let received_header = Header::try_from(wrong_version);
        assert_eq!(Err(HeaderParsingError::WrongVersion), received_header);
    }

    #[test]
    fn test_serialize_deserialize() {
        let header = Header::new(10, 0);
        let header_bytes: Vec<u8> = header.into();
        let received_header = Header::try_from(&header_bytes[..]).unwrap();

        assert_eq!(header, received_header);
    }
}
