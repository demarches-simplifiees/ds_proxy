#[derive(Debug, Clone, Copy)]
pub enum DecipherType {
    Encrypted { chunk_size: usize },
    Plaintext,
}
