//! Namespace-level functions for Bark FFI

use anyhow::Context;
use bark::ark;
use bip39::Mnemonic;

use crate::error::BarkError;

/// Generate a new 12-word BIP39 mnemonic
#[boltffi::export]
pub fn generate_mnemonic() -> Result<String, BarkError> {
    let mnemonic = Mnemonic::generate(12).context("Failed to generate mnemonic")?;
    Ok(mnemonic.to_string())
}

/// Validate a BIP39 mnemonic phrase
#[boltffi::export]
pub fn validate_mnemonic(mnemonic: String) -> Result<bool, BarkError> {
    match Mnemonic::parse(mnemonic.trim()) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Validate an Ark address (basic format check only)
///
/// This only validates the format of the address, not whether it belongs
/// to a specific Ark server. For full validation against a connected server,
/// use Wallet::validate_arkoor_address() instead.
#[boltffi::export]
pub fn validate_ark_address(address: String) -> Result<bool, BarkError> {
    match address.parse::<ark::Address>() {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Extract a signed transaction from a PSBT
///
/// Takes a base64-encoded PSBT and extracts the final signed transaction.
/// This is useful after signing a PSBT (e.g., from drain_exits) before broadcasting.
///
/// # Arguments
///
/// * `psbt_base64` - Base64-encoded PSBT string
///
/// # Returns
///
/// Hex-encoded signed transaction ready for broadcasting
#[boltffi::export]
pub fn extract_tx_from_psbt(psbt_base64: String) -> Result<String, BarkError> {
    use bitcoin::consensus::encode::serialize_hex;
    use bitcoin::psbt::Psbt;
    use std::str::FromStr;

    let psbt = Psbt::from_str(&psbt_base64).context("invalid PSBT")?;

    let tx = psbt.extract_tx().context("Failed to extract transaction")?;

    Ok(serialize_hex(&tx))
}

/// Default for `Config.vtxo_key_gap_limit`: the run of consecutive unused
/// seed-derived VTXO key indices a scan crosses before concluding a VTXO
/// isn't ours.
#[boltffi::export]
pub fn default_vtxo_key_gap_limit() -> u32 {
    bark::DEFAULT_VTXO_KEY_GAP_LIMIT
}

/// Largest value `Config.vtxo_key_gap_limit` (or an import/recovery
/// `gap_limit` override) accepts. A scan that matches nothing runs the limit
/// to its end, so an unbounded limit is unbounded work.
#[boltffi::export]
pub fn max_vtxo_key_gap_limit() -> u32 {
    bark::MAX_VTXO_KEY_GAP_LIMIT
}
