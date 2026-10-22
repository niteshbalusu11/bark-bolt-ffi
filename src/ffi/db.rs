use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context};
use log::{info, warn};

use bark::persist::sqlite::{SqliteClient, DEFAULT_DB_FILE};

/// Check identity before opening Bark's migrating connection or creating BDK tables.
pub(crate) fn validate_existing_wallet(
    datadir: &str,
    network: crate::Network,
    mnemonic_or_seed: &str,
) -> anyhow::Result<()> {
    let path = sqlite_path(datadir);
    if !path.is_file() {
        bail!("existing wallet database is missing");
    }
    let network: bitcoin::Network = network.into();
    let seed = crate::core::wallet::seed_from_str(network, mnemonic_or_seed)?;
    let nonempty = |suffix: &str| {
        std::fs::metadata(format!("{}{suffix}", path.display()))
            .map(|m| m.len() > 0)
            .unwrap_or(false)
    };
    let mut header = [0; 20];
    std::fs::File::open(&path)?
        .read_exact(&mut header)
        .context("existing wallet database is empty or truncated")?;
    let wal_mode = header[18] == 2 || header[19] == 2;
    // An immutable connection avoids creating sidecars for a closed WAL database.
    // A live WAL needs its existing shared-memory index; never create one here.
    let wal = PathBuf::from(format!("{}-wal", path.display())).exists();
    if (wal_mode && nonempty("-journal")) || (wal && !nonempty("-shm")) {
        bail!("existing wallet requires SQLite journal recovery before identity validation");
    }
    let encoded: String = path
        .to_str()
        .context("invalid wallet database path")?
        .bytes()
        .map(|b| match b {
            b'/' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect();
    let uri = format!(
        "file:{encoded}?mode=ro{}",
        if wal_mode && !wal { "&immutable=1" } else { "" }
    );
    let connection = rusqlite::Connection::open_with_flags(
        uri,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )?;
    let (stored_network, stored_fingerprint): (String, String) = connection
        .query_row(
            "SELECT network, fingerprint FROM bark_properties WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| {
            let context = match &error {
                rusqlite::Error::QueryReturnedNoRows => {
                    "existing database does not contain initialized Bark wallet properties"
                }
                rusqlite::Error::SqliteFailure(code, Some(message))
                    if code.extended_code == rusqlite::ffi::SQLITE_ERROR
                        && message == "no such table: bark_properties" =>
                {
                    "existing database does not contain initialized Bark wallet properties"
                }
                rusqlite::Error::SqliteFailure(code, _)
                    if code.extended_code == rusqlite::ffi::SQLITE_READONLY_ROLLBACK
                        || code.extended_code == rusqlite::ffi::SQLITE_READONLY_RECOVERY =>
                {
                    "existing wallet requires SQLite journal recovery before identity validation"
                }
                _ => "failed to read existing Bark wallet properties",
            };
            anyhow::Error::new(error).context(context)
        })?;
    if stored_network.parse::<bitcoin::Network>()? != network {
        bail!("existing wallet network does not match requested network");
    }
    if stored_fingerprint.parse::<bitcoin::bip32::Fingerprint>()? != seed.fingerprint() {
        bail!("incorrect mnemonic for existing wallet");
    }
    Ok(())
}

/// Open a SQLite database connection.
///
/// # Arguments
///
/// * `datadir` - The directory containing the wallet data
///
/// # Returns
///
/// An `Arc<SqliteClient>` that can be shared across multiple wallet instances.
pub fn open_sqlite_db(datadir: &str) -> anyhow::Result<Arc<SqliteClient>> {
    let datadir_path = PathBuf::from(datadir);

    // Ensure the directory exists
    std::fs::create_dir_all(&datadir_path)
        .with_context(|| format!("Failed to create datadir {}", datadir_path.display(),))?;

    let db_path = sqlite_path(datadir);

    // Open new connection
    info!("[DB] Opening new connection for {}", db_path.display());
    let db = Arc::new(
        SqliteClient::open(&db_path)
            .with_context(|| format!("Failed to open database at {}", db_path.display(),))?,
    );
    Ok(db)
}

pub(crate) fn sqlite_path(datadir: &str) -> PathBuf {
    let datadir_path = PathBuf::from(datadir);
    let db_path = datadir_path.join(DEFAULT_DB_FILE);

    // bark-ffi pre v0.3.0 used bark.sqlite as the db filename,
    let compat_db_path = datadir_path.join("bark.sqlite");

    if !db_path.exists() && compat_db_path.exists() {
        warn!(
            "You're using the old default filename bark.sqlite for your SQLite DB, \
                while the new default is db.sqlite. You might want to rename your file."
        );
        compat_db_path
    } else {
        db_path
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ffi::{
            onchain::OnchainWallet,
            wallet::{Wallet, WalletOpenArgs},
        },
        Network,
    };
    const MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    fn contents(directory: &std::path::Path) -> std::collections::BTreeMap<String, Vec<u8>> {
        std::fs::read_dir(directory)
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                (
                    path.file_name().unwrap().to_string_lossy().into(),
                    std::fs::read(path).unwrap(),
                )
            })
            .collect()
    }

    #[test]
    fn existing_opens_reject_without_writes() {
        crate::ffi::runtime::TOKIO_RT.block_on(async {
            for case in ["empty", "foreign", "network", "mnemonic", "missing"] {
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join("db.sqlite");
                match case {
                    "empty" => std::fs::write(&path, []).unwrap(),
                    "foreign" => { let c = rusqlite::Connection::open(&path).unwrap(); c.execute_batch("CREATE TABLE foreign_data(value); INSERT INTO foreign_data VALUES(42)").unwrap(); },
                    "missing" => {},
                    _ => { std::fs::copy("tests/fixtures/noah-v2-schema42.sqlite",&path).unwrap(); },
                }
                let before = contents(directory.path());
                let network = if case == "network" { Network::Bitcoin } else { Network::Regtest };
                let mnemonic = if case == "mnemonic" { crate::generate_mnemonic().unwrap() } else { MNEMONIC.into() };
                let config = || serde_json::from_value(serde_json::json!({"server_address":"http://127.0.0.1:1","esplora_address":"http://127.0.0.1:1"})).unwrap();
                let datadir = directory.path().to_str().unwrap().to_string();
                assert!(OnchainWallet::open_existing(network,mnemonic.clone(),config(),datadir.clone()).await.is_err(),"{case}");
                assert!(before == contents(directory.path()),"onchain open changed {case}");
                assert!(Wallet::open(network,mnemonic,config(),WalletOpenArgs { run_daemon:false,datadir,create_if_not_exists:false,create_without_server:false,skip_recovery:true }).await.is_err(),"{case}");
                assert!(before == contents(directory.path()),"wallet open changed {case}");
            }
        });
    }

    #[test]
    fn readonly_preflight_uses_wal_identity_and_preserves_closed_wal_files() {
        let directory = tempfile::Builder::new()
            .prefix("bark ?#% ")
            .tempdir()
            .unwrap();
        let path = directory.path().join("db.sqlite");
        std::fs::copy("tests/fixtures/noah-v2-schema42.sqlite", &path).unwrap();
        let writer = rusqlite::Connection::open(&path).unwrap();
        writer
            .execute_batch(
                "PRAGMA journal_mode=WAL; UPDATE bark_properties SET fingerprint='00000000'",
            )
            .unwrap();
        let before = std::fs::read(&path).unwrap();
        let names = || {
            std::fs::read_dir(directory.path())
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect::<std::collections::BTreeSet<_>>()
        };
        let before_names = names();
        assert!(super::validate_existing_wallet(
            directory.path().to_str().unwrap(),
            Network::Regtest,
            MNEMONIC
        )
        .unwrap_err()
        .to_string()
        .contains("incorrect mnemonic"));
        assert!(before == std::fs::read(&path).unwrap());
        assert_eq!(before_names, names());
        drop(writer);
        let before = contents(directory.path());
        assert!(super::validate_existing_wallet(
            directory.path().to_str().unwrap(),
            Network::Regtest,
            MNEMONIC
        )
        .is_err());
        assert!(before == contents(directory.path()));
    }

    #[test]
    fn persist_journal_wallet_identity_still_validates() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("db.sqlite");
        std::fs::copy("tests/fixtures/noah-v2-schema42.sqlite", &path).unwrap();
        let writer = rusqlite::Connection::open(&path).unwrap();
        writer
            .execute_batch(
                "PRAGMA journal_mode=PERSIST; UPDATE bark_properties SET network='signet'; UPDATE bark_properties SET network='regtest'",
            )
            .unwrap();
        assert!(
            std::fs::metadata(directory.path().join("db.sqlite-journal"))
                .unwrap()
                .len()
                > 0
        );
        let before = contents(directory.path());
        super::validate_existing_wallet(
            directory.path().to_str().unwrap(),
            Network::Regtest,
            MNEMONIC,
        )
        .unwrap();
        assert!(before == contents(directory.path()));
        assert!(super::validate_existing_wallet(
            directory.path().to_str().unwrap(),
            Network::Bitcoin,
            MNEMONIC
        )
        .is_err());
        assert!(before == contents(directory.path()));
    }

    #[test]
    fn hot_journal_error_is_actionable_and_does_not_recover_input() {
        let source = tempfile::tempdir().unwrap();
        let source_path = source.path().join("db.sqlite");
        std::fs::copy("tests/fixtures/noah-v2-schema42.sqlite", &source_path).unwrap();
        let writer = rusqlite::Connection::open(&source_path).unwrap();
        writer
            .execute_batch("BEGIN IMMEDIATE; UPDATE bark_properties SET network='signet'")
            .unwrap();
        writer.cache_flush().unwrap();
        let directory = tempfile::tempdir().unwrap();
        std::fs::copy(&source_path, directory.path().join("db.sqlite")).unwrap();
        std::fs::copy(
            source.path().join("db.sqlite-journal"),
            directory.path().join("db.sqlite-journal"),
        )
        .unwrap();
        writer.execute_batch("ROLLBACK").unwrap();
        let before = contents(directory.path());
        let error = super::validate_existing_wallet(
            directory.path().to_str().unwrap(),
            Network::Regtest,
            MNEMONIC,
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("requires SQLite journal recovery before identity validation"),
            "{error:#}"
        );
        assert!(before == contents(directory.path()));
    }

    #[test]
    fn empty_header_error_is_actionable() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("db.sqlite"), []).unwrap();
        let before = contents(directory.path());
        let error = super::validate_existing_wallet(
            directory.path().to_str().unwrap(),
            Network::Regtest,
            MNEMONIC,
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("empty or truncated"),
            "{error:#}"
        );
        assert!(before == contents(directory.path()));
    }

    #[test]
    fn zero_length_wal_requires_existing_index_without_creating_one() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("db.sqlite");
        std::fs::copy("tests/fixtures/noah-v2-schema42.sqlite", &path).unwrap();
        let writer = rusqlite::Connection::open(&path).unwrap();
        writer
            .execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_checkpoint(TRUNCATE)")
            .unwrap();
        assert_eq!(
            std::fs::metadata(directory.path().join("db.sqlite-wal"))
                .unwrap()
                .len(),
            0
        );
        let before = contents(directory.path());
        super::validate_existing_wallet(
            directory.path().to_str().unwrap(),
            Network::Regtest,
            MNEMONIC,
        )
        .unwrap();
        assert!(before == contents(directory.path()));
        drop(writer);
        std::fs::write(directory.path().join("db.sqlite-wal"), []).unwrap();
        let before = contents(directory.path());
        let error = super::validate_existing_wallet(
            directory.path().to_str().unwrap(),
            Network::Regtest,
            MNEMONIC,
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("requires SQLite journal recovery"),
            "{error:#}"
        );
        assert!(before == contents(directory.path()));
    }

    #[test]
    fn schema42_identity_preflight_and_legacy_filename() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bark.sqlite");
        std::fs::copy("tests/fixtures/noah-v2-schema42.sqlite", &path).unwrap();
        let before = contents(directory.path());
        super::validate_existing_wallet(
            directory.path().to_str().unwrap(),
            Network::Regtest,
            MNEMONIC,
        )
        .unwrap();
        assert!(before == contents(directory.path()));
    }
}

#[cfg(test)]
mod positive_tests {
    #[test]
    fn schema42_wallet_still_opens() {
        if std::env::var("BARK_REGTEST").as_deref() != Ok("1") {
            return;
        }
        crate::ffi::runtime::TOKIO_RT.block_on(async {
            let directory = tempfile::tempdir().unwrap();
            std::fs::copy("tests/fixtures/noah-v2-schema42.sqlite",directory.path().join("db.sqlite")).unwrap();
            let config = || serde_json::from_value(serde_json::json!({"server_address":crate::ffi::regtest_url("ARK_PORT", "3635"),"bitcoind_address":crate::ffi::regtest_url("BITCOIN_PORT", "19443"),"bitcoind_user":"second","bitcoind_pass":"ark"})).unwrap();
            let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_string();
            let datadir = directory.path().to_str().unwrap().to_string();
            let onchain = crate::ffi::onchain::OnchainWallet::open_existing(crate::Network::Regtest,mnemonic.clone(),config(),datadir.clone()).await.unwrap();
            let wallet = crate::ffi::wallet::Wallet::open_with_onchain(crate::Network::Regtest,mnemonic,config(),crate::ffi::wallet::WalletOpenArgs { run_daemon:false,datadir,create_if_not_exists:false,create_without_server:false,skip_recovery:true },&onchain).await.unwrap();
            assert!(!wallet.fingerprint().is_empty());
        });
    }
}
