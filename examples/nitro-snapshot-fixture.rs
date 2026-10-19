//! Public test-only schema-42 wallet. The migration SQL matches bark-0.7.1,
//! Nitro v0.0.144's pin (only an upstream migration unit test changed).
use bark::persist::{sqlite::SqliteClient, BarkPersister};
fn main() {
    let path = "tests/fixtures/noah-v2-schema42.sqlite";
    assert!(
        !std::path::Path::new(path).exists(),
        "never overwrite a fixture"
    );
    let db = SqliteClient::open(path).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let mnemonic = bip39::Mnemonic::parse("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about").unwrap();
        let seed = bark::WalletSeed::new_from_mnemonic(bitcoin::Network::Regtest, &mnemonic);
        db.init_wallet(&bark::WalletProperties { network: bitcoin::Network::Regtest, fingerprint: seed.fingerprint(), server_pubkey: Some("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798".parse().unwrap()), server_mailbox_pubkey: Some("02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5".parse().unwrap()) }).await.unwrap();
        db.initialize_bdk_wallet().await.unwrap();
    });
}
