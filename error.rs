//! Comprehensive error types for the Solana MEV bot

use thiserror::Error;

/// Main error type for the MEV bot
#[derive(Error, Debug)]
pub enum BotError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("RPC client error: {0}")]
    Rpc(String),

    #[error("Account fetch error: {0}")]
    AccountFetch(String),

    #[error("Pool parsing error: {0}")]
    PoolParse(String),

    #[error("Price calculation error: {0}")]
    PriceCalculation(String),

    #[error("Transaction building error: {0}")]
    Transaction(String),

    #[error("DEX operation error: {0}")]
    Dex(String),

    #[error("Cache operation error: {0}")]
    Cache(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, BotError>;

/// Convert solana_client errors to BotError
impl From<solana_client::client_error::ClientError> for BotError {
    fn from(err: solana_client::client_error::ClientError) -> Self {
        BotError::Rpc(err.to_string())
    }
}

/// Convert Pubkey parsing errors to BotError
impl From<solana_sdk::pubkey::ParsePubkeyError> for BotError {
    fn from(err: solana_sdk::pubkey::ParsePubkeyError) -> Self {
        BotError::Parse(format!("Invalid public key: {}", err))
    }
}

/// Convert serde_json errors to BotError
impl From<serde_json::Error> for BotError {
    fn from(err: serde_json::Error) -> Self {
        BotError::Parse(format!("JSON parsing error: {}", err))
    }
}
