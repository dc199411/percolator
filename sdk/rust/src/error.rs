//! SDK Error types

use thiserror::Error;

/// SDK errors
#[derive(Error, Debug)]
pub enum PercolatorSdkError {
    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Insufficient funds: required {required}, available {available}")]
    InsufficientFunds { required: u64, available: u64 },

    #[error("Portfolio not initialized")]
    PortfolioNotInitialized,

    #[error("Position not found")]
    PositionNotFound,

    #[error("Invalid side")]
    InvalidSide,

    #[error("Invalid time in force")]
    InvalidTimeInForce,

    #[error("Order exceeds margin")]
    OrderExceedsMargin,

    #[error("Portfolio liquidatable")]
    PortfolioLiquidatable,

    #[error("Insurance withdrawal locked until {unlock_ts}")]
    WithdrawalLocked { unlock_ts: u64 },

    #[error("Program error: {code}")]
    ProgramError { code: u32 },

    #[error("Solana SDK error: {0}")]
    SolanaError(#[from] solana_sdk::pubkey::PubkeyError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result type for SDK operations
pub type Result<T> = std::result::Result<T, PercolatorSdkError>;
