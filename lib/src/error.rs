use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlindHopError {
    #[error("Sphinx error: {0}")]
    Sphinx(#[from] SphinxError),

    #[error("Configuration error: {0}")]
    Config(String),
}

#[derive(Error, Debug)]
pub enum SphinxError {
    #[error("payload too large: {size} bytes, max {max}")]
    PayloadTooLarge { size: usize, max: usize },

    #[error("invalid hop count: {0} (must be 1-5)")]
    InvalidHopCount(u8),

    #[error("empty route: at least one hop required")]
    EmptyRoute,

    #[error("header MAC verification failed at hop {hop}")]
    MacVerificationFailed { hop: usize },

    #[error("decryption failed: wrong secret key or corrupted packet")]
    DecryptionFailed,

    #[error("invalid packet size: expected {expected}, got {actual}")]
    InvalidPacketSize { expected: usize, actual: usize },

    #[error("SURB processing error: {0}")]
    SurbError(String),
}

pub type Result<T> = std::result::Result<T, BlindHopError>;
pub type SphinxResult<T> = std::result::Result<T, SphinxError>;
