use std::fmt;

/// Stable error categories; messages retain the underlying diagnostic.
#[boltffi::data]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarkErrorCode {
    Other,
    InvalidAddress,
    InvalidAmount,
    InvalidFeeRate,
    InsufficientFunds,
    Dust,
    PreparedTransactionNotFound,
    PreparedInputsUnavailable,
    UnsupportedOperation,
    Persistence,
}

/// Error message including the complete underlying cause chain.
#[boltffi::error]
#[derive(Clone, Debug)]
pub struct BarkError {
    pub message: String,
    #[boltffi::default(BarkErrorCode::Other)]
    pub code: BarkErrorCode,
}

impl BarkError {
    pub(crate) fn typed(code: BarkErrorCode, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code,
        }
    }

    pub fn new(error: impl Into<anyhow::Error>) -> Self {
        Self::from(error.into())
    }

    pub fn message(&self) -> String {
        self.message.clone()
    }
}

impl fmt::Display for BarkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for BarkError {}
impl From<anyhow::Error> for BarkError {
    fn from(error: anyhow::Error) -> Self {
        Self {
            message: format!("{error:#}"),
            code: error
                .downcast_ref::<bark::onchain::bdk_wallet::error::CreateTxError>()
                .map_or(BarkErrorCode::Other, construction_error_code),
        }
    }
}
impl From<String> for BarkError {
    fn from(message: String) -> Self {
        Self {
            message,
            code: BarkErrorCode::Other,
        }
    }
}

pub(crate) fn construction_error_code(
    error: &bark::onchain::bdk_wallet::error::CreateTxError,
) -> BarkErrorCode {
    use bark::onchain::bdk_wallet::error::CreateTxError;
    match error {
        CreateTxError::CoinSelection(_) | CreateTxError::NoUtxosSelected => {
            BarkErrorCode::InsufficientFunds
        }
        CreateTxError::OutputBelowDustLimit(_) => BarkErrorCode::Dust,
        _ => BarkErrorCode::Other,
    }
}
impl From<&str> for BarkError {
    fn from(message: &str) -> Self {
        Self::from(message.to_owned())
    }
}
impl From<boltffi::UnexpectedFfiCallbackError> for BarkError {
    fn from(error: boltffi::UnexpectedFfiCallbackError) -> Self {
        Self::from(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_message() {
        let err = BarkError::from(anyhow::anyhow!("something broke"));
        assert_eq!(err.message(), "something broke");
    }

    #[test]
    fn shows_full_cause_chain() {
        // Display uses the alternate `{:#}` form, which joins the anyhow chain.
        let err = BarkError::from(anyhow::anyhow!("root cause").context("outer context"));
        let msg = err.message();
        assert!(msg.contains("outer context"), "{msg}");
        assert!(msg.contains("root cause"), "{msg}");
    }
}
