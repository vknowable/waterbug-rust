use std::fmt;
use thiserror::Error;

#[derive(uniffi::Error, Error, Debug)]
pub enum WaterbugError {
    #[error("Tendermint RPC error: {0}")]
    TendermintError(String),

    #[error("ChainId parse error: {0}")]
    ChainIdParseError(String),

    #[error("NamadaSdk error: {0}")]
    NamadaSdkError(String),

    #[error("Sdk not initialized")]
    SdkNotInitError,
}

impl From<tendermint_rpc::Error> for WaterbugError {
    fn from(err: tendermint_rpc::Error) -> Self {
        WaterbugError::TendermintError(err.to_string())
    }
}

impl From<namada_sdk::chain::ChainIdParseError> for WaterbugError {
  fn from(err: namada_sdk::chain::ChainIdParseError) -> Self {
      WaterbugError::ChainIdParseError(err.to_string())
  }
}

impl From<namada_sdk::error::Error> for WaterbugError {
  fn from(err: namada_sdk::error::Error) -> Self {
      WaterbugError::NamadaSdkError(err.to_string())
  }
}