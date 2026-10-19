//! Reproduce Nitro v0.0.144 formulas without calling this crate's crypto exports.
//! Public BIP39 test vector only; never use this seed for funds.
use bitcoin::{
    bip32::{ChildNumber, Xpriv},
    hashes::Hash,
    secp256k1::{Message, Secp256k1},
    Network,
};
fn main() {
    let mnemonic = bip39::Mnemonic::parse("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about").unwrap();
    let message = "Noah native compatibility 🦀";
    let secp = Secp256k1::new();
    let hash =
        Message::from_digest(bitcoin::sign_message::signed_msg_hash(message).to_byte_array());
    let mut vectors = vec![];
    for index in [0_u32, 1, 7, 0x7fffffff, 0x80000000, 0xffffffff] {
        let master = Xpriv::new_master(Network::Bitcoin, &mnemonic.to_seed("")).unwrap();
        let offline = master
            .derive_priv(&secp, &[350.into()])
            .unwrap()
            .derive_priv(&secp, &[index.into()])
            .unwrap()
            .to_keypair(&secp);
        let wallet = master
            .derive_priv(
                &secp,
                &[
                    ChildNumber::from_hardened_idx(350).unwrap(),
                    ChildNumber::from_hardened_idx(0).unwrap(),
                    index.into(),
                ],
            )
            .unwrap()
            .to_keypair(&secp);
        vectors.push(serde_json::json!({"index":index,"offlinePublicKey":offline.public_key().to_string(),"offlineSignature":secp.sign_ecdsa(&hash,&offline.secret_key()).to_string(),"walletPublicKey":wallet.public_key().to_string(),"walletSignature":secp.sign_ecdsa(&hash,&wallet.secret_key()).to_string()}));
    }
    let output = serde_json::json!({"reference":"Nitro v0.0.144 (10bcf3ec9325696601de10f833fab747f0a5ec4c), identical pure Rust BIP32 + RFC6979", "mnemonic":mnemonic.to_string(),"message":message,"vectors":vectors});
    std::fs::write(
        "tests/fixtures/nitro-v0.0.144-crypto.json",
        serde_json::to_vec_pretty(&output).unwrap(),
    )
    .unwrap();
}
