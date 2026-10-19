use anyhow::Context;
use bip39::Mnemonic;
use bitcoin::bip32::Xpriv;
use bitcoin::hashes::Hash;
use bitcoin::secp256k1::{ecdsa::Signature, Keypair, Message, PublicKey, Secp256k1};

use crate::{BarkError, Network};

fn mnemonic_key(mnemonic: &str, network: Network, index: u32) -> anyhow::Result<Keypair> {
    let mnemonic = Mnemonic::parse(mnemonic).context("invalid mnemonic")?;
    let secp = Secp256k1::new();
    // Nitro login keys deliberately use m/350/index, separate from Bark wallet keys.
    Ok(
        Xpriv::new_master(bitcoin::Network::from(network), &mnemonic.to_seed(""))?
            .derive_priv(&secp, &[350.into()])?
            .derive_priv(&secp, &[index.into()])?
            .to_keypair(&secp),
    )
}

fn message_hash(message: &str) -> Message {
    Message::from_digest(bitcoin::sign_message::signed_msg_hash(message).to_byte_array())
}

pub(crate) fn sign(message: &str, key: &Keypair) -> String {
    Secp256k1::new()
        .sign_ecdsa(&message_hash(message), &key.secret_key())
        .to_string()
}

/// Nitro v0.0.144 login key: m/350/index (empty BIP39 passphrase).
/// Indices >= 2^31 select hardened children with the high bit removed.
#[boltffi::export]
pub fn derive_public_key_from_mnemonic(
    mnemonic: String,
    network: Network,
    index: u32,
) -> Result<String, BarkError> {
    Ok(mnemonic_key(&mnemonic, network, index)?
        .public_key()
        .to_string())
}

/// Bitcoin signed-message hash, deterministic ECDSA, lowercase DER hex.
/// Uses Nitro's offline m/350/index path, including high-bit hardened indices.
#[boltffi::export]
pub fn sign_message_with_mnemonic(
    message: String,
    mnemonic: String,
    network: Network,
    index: u32,
) -> Result<String, BarkError> {
    Ok(sign(&message, &mnemonic_key(&mnemonic, network, index)?))
}

/// False for a validly encoded signature that does not match. Malformed DER
/// or public-key hex returns an error, as in Nitro's bridge parser.
#[boltffi::export]
pub fn verify_message(
    message: String,
    signature_der_hex: String,
    public_key_hex: String,
) -> Result<bool, BarkError> {
    let signature: Signature = signature_der_hex.parse().context("invalid DER signature")?;
    let public_key: PublicKey = public_key_hex.parse().context("invalid public key")?;
    Ok(Secp256k1::new()
        .verify_ecdsa(&message_hash(&message), &signature, &public_key)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nitro_golden_vectors_all_networks_and_hardened_boundaries() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/nitro-v0.0.144-crypto.json"))
                .unwrap();
        let mnemonic = fixture["mnemonic"].as_str().unwrap();
        let message = fixture["message"].as_str().unwrap();
        for network in [
            Network::Bitcoin,
            Network::Testnet,
            Network::Signet,
            Network::Regtest,
        ] {
            for vector in fixture["vectors"].as_array().unwrap() {
                let index = vector["index"].as_u64().unwrap() as u32;
                let key = derive_public_key_from_mnemonic(mnemonic.into(), network, index).unwrap();
                let signature =
                    sign_message_with_mnemonic(message.into(), mnemonic.into(), network, index)
                        .unwrap();
                assert_eq!(key, vector["offlinePublicKey"].as_str().unwrap());
                assert_eq!(signature, vector["offlineSignature"].as_str().unwrap());
                assert_ne!(key, vector["walletPublicKey"].as_str().unwrap());
                assert!(verify_message(message.into(), signature.clone(), key.clone()).unwrap());
                assert!(!verify_message(format!("{message}!"), signature, key).unwrap());
                assert_eq!(
                    bitcoin::bip32::ChildNumber::from(index).is_hardened(),
                    index >= (1 << 31)
                );
            }
        }
        assert!(derive_public_key_from_mnemonic("invalid".into(), Network::Regtest, 0).is_err());
        assert!(verify_message(message.into(), "00".into(), "00".into()).is_err());
    }
}
