#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DecipherType {
    Encrypted { chunk_size: usize },
    Plaintext,
}
