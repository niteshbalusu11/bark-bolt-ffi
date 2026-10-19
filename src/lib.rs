//! Swift and Kotlin bindings for the native Bark wallet.
mod backup;
mod config;
mod core;
mod crypto;
mod error;
mod ffi;
mod functions;
mod types;

pub use backup::{validate_wallet_snapshot, WalletSnapshotExpectation, WalletSnapshotInfo};
pub use config::Config;
pub use crypto::{derive_public_key_from_mnemonic, sign_message_with_mnemonic, verify_message};
pub use error::BarkError;
pub use ffi::*;
pub use functions::*;
pub use types::*;
