//! Adapted from Nitro Ark v0.0.144 bark-cpp/src/backup.rs (MIT).
use crate::BarkError;
use anyhow::{bail, Context};
use bark::persist::sqlite::SqliteClient;
use bark::persist::BarkPersister;
use bark::WalletProperties;
use bitcoin::hashes::{sha256, Hash, HashEngine};
use rusqlite::backup::{Backup, StepResult};
use rusqlite::{Connection, OpenFlags};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, Weak};
use std::time::{Duration, Instant};
use tempfile::{Builder, NamedTempFile};

const BACKUP_PAGES_PER_STEP: i32 = 128;
const BACKUP_STEP_PAUSE: Duration = Duration::from_millis(10);
const BACKUP_BUSY_TIMEOUT: Duration = Duration::from_secs(30);
const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(10);
static SNAPSHOT_MUTEX: Mutex<()> = Mutex::new(());
static LIVE_DATABASES: LazyLock<Mutex<Vec<Weak<PathBuf>>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

#[boltffi::data]
#[derive(Debug)]
pub struct WalletSnapshotInfo {
    pub path: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub network: String,
    pub wallet_fingerprint: String,
    pub server_pubkey: Option<String>,
    /// The server mailbox pubkey, not this wallet's mailbox key.
    pub mailbox_pubkey: Option<String>,
    pub schema_version: u32,
}

#[boltffi::data]
#[derive(Default)]
pub struct WalletSnapshotExpectation {
    #[boltffi::default(None)]
    pub network: Option<String>,
    #[boltffi::default(None)]
    pub wallet_fingerprint: Option<String>,
    #[boltffi::default(None)]
    pub server_pubkey: Option<String>,
}

pub(crate) fn register_live_database(path: &Path) -> anyhow::Result<Arc<PathBuf>> {
    let path = Arc::new(path.canonicalize()?);
    let mut live = LIVE_DATABASES.lock().unwrap();
    live.retain(|entry| entry.strong_count() > 0);
    live.push(Arc::downgrade(&path));
    Ok(path)
}

/// Validate a plaintext db.sqlite from a Noah v2 backup. The input is only read;
/// schema migration and typed Bark loading happen in a private temporary copy.
#[boltffi::export]
pub async fn validate_wallet_snapshot(
    path: String,
    #[boltffi::default(None)] expected: Option<WalletSnapshotExpectation>,
) -> Result<WalletSnapshotInfo, BarkError> {
    crate::ffi::runtime::run_async(async move {
        tokio::task::spawn_blocking(move || {
            validate_wallet_snapshot_blocking(Path::new(&path), expected)
        })
        .await
        .context("snapshot validation worker failed")?
        .map_err(BarkError::from)
    })
    .await
}

pub(crate) fn create_wallet_snapshot_blocking(
    source: &Path,
    destination: &Path,
) -> anyhow::Result<WalletSnapshotInfo> {
    let _snapshot_guard = SNAPSHOT_MUTEX
        .lock()
        .map_err(|_| anyhow::anyhow!("wallet snapshot lock is poisoned"))?;

    validate_destination(source, destination)?;
    let partial = temporary_snapshot_file(destination)?;
    sqlite_backup(source, partial.path())?;
    let info = inspect_snapshot(partial.path(), destination)?;
    validate_sqlite_integrity(partial.path())?;
    partial
        .as_file()
        .sync_all()
        .context("failed to flush wallet snapshot")?;
    partial
        .persist_noclobber(destination)
        .map_err(|error| error.error)
        .with_context(|| {
            format!(
                "failed to publish wallet snapshot at {}",
                destination.display()
            )
        })?;

    File::open(parent_directory(destination))?
        .sync_all()
        .context("failed to flush snapshot directory")?;
    Ok(info)
}

fn validate_wallet_snapshot_blocking(
    snapshot: &Path,
    expected: Option<WalletSnapshotExpectation>,
) -> anyhow::Result<WalletSnapshotInfo> {
    ensure_regular_snapshot_file(snapshot)?;
    ensure_self_contained_snapshot(snapshot)?;
    let mut input = File::open(snapshot).context("failed to open snapshot")?;
    {
        let live = LIVE_DATABASES.lock().unwrap();
        for active in live.iter().filter_map(Weak::upgrade) {
            if paths_refer_to_same_file(snapshot, &active)? {
                bail!("Refusing to validate the live wallet database; create a snapshot first");
            }
        }
    }
    // Use the app-writable input directory (Android may not have a system temp).
    let parent = parent_directory(snapshot);
    let mut migrated = Builder::new()
        .prefix(".bark-validation-")
        .suffix(".sqlite")
        .tempfile_in(parent)
        .context("failed to create validation database")?;
    std::io::copy(&mut input, migrated.as_file_mut())?;
    ensure_self_contained_snapshot(snapshot)?;
    validate_sqlite_integrity(migrated.path())?;
    let source_schema_version = read_schema_version(migrated.path())?;
    let supported = supported_schema_version(parent)?;
    if source_schema_version > supported {
        bail!(
            "Snapshot schema version {} is newer than supported version {}",
            source_schema_version,
            supported
        );
    }
    let db = SqliteClient::open(migrated.path())
        .context("snapshot could not be opened or migrated by Bark")?;
    let properties = crate::ffi::runtime::TOKIO_RT
        .block_on(async {
            let properties = db
                .read_properties()
                .await?
                .context("snapshot does not contain initialized wallet properties")?;
            db.get_all_vtxos().await?;
            db.get_all_movements().await?;
            for id in db.get_pending_round_state_ids().await? {
                db.get_round_state_by_id(id)
                    .await?
                    .context("missing round state")?;
            }
            db.get_all_wallet_action_checkpoints().await?;
            db.get_exit_vtxo_entries().await?;
            let _ = db.initialize_bdk_wallet().await?;
            anyhow::Ok(properties)
        })
        .context("snapshot contains unreadable Bark wallet data")?;

    validate_expected_identity(&properties, expected.as_ref())?;

    snapshot_info(snapshot, snapshot, &properties, source_schema_version)
}

fn validate_destination(source: &Path, destination: &Path) -> anyhow::Result<()> {
    if destination.exists() {
        bail!(
            "Snapshot destination already exists: {}",
            destination.display()
        );
    }
    if paths_refer_to_same_file(source, destination)? {
        bail!("Snapshot destination cannot be the live wallet database");
    }

    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if !parent.is_dir() {
        bail!(
            "Snapshot destination parent does not exist: {}",
            parent.display()
        );
    }
    Ok(())
}

fn ensure_regular_snapshot_file(path: &Path) -> anyhow::Result<()> {
    let metadata = path
        .metadata()
        .with_context(|| format!("snapshot does not exist: {}", path.display()))?;
    if !metadata.is_file() {
        bail!("Snapshot path is not a regular file: {}", path.display());
    }
    if metadata.len() == 0 {
        bail!("Snapshot file is empty: {}", path.display());
    }
    Ok(())
}

fn ensure_self_contained_snapshot(path: &Path) -> anyhow::Result<()> {
    for suffix in ["-wal", "-journal"] {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        match std::fs::metadata(Path::new(&sidecar)) {
            Ok(metadata) if metadata.len() > 0 => bail!(
                "Expected a self-contained snapshot; nonempty {} sidecar found. Use createSnapshot to export a checkpointed database first",
                suffix
            ),
            Ok(_) => {},
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
            Err(error) => return Err(error).context("failed to inspect snapshot sidecar"),
        }
    }
    Ok(())
}

fn paths_refer_to_same_file(left: &Path, right: &Path) -> anyhow::Result<bool> {
    if left == right {
        return Ok(true);
    }
    if left.exists() && right.exists() {
        return same_file(left, right);
    }
    Ok(false)
}

fn temporary_snapshot_file(destination: &Path) -> anyhow::Result<NamedTempFile> {
    let parent = parent_directory(destination);
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("wallet-snapshot.sqlite");
    Builder::new()
        .prefix(&format!(".{file_name}.partial-"))
        .tempfile_in(parent)
        .context("failed to create partial wallet snapshot")
}

fn sqlite_backup(source: &Path, destination: &Path) -> anyhow::Result<()> {
    let source_connection = Connection::open_with_flags(
        source,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("failed to open source database {}", source.display()))?;
    source_connection.busy_timeout(BACKUP_STEP_PAUSE)?;

    let mut destination_connection = Connection::open(destination).with_context(|| {
        format!(
            "failed to open destination database {}",
            destination.display()
        )
    })?;
    destination_connection.busy_timeout(BACKUP_STEP_PAUSE)?;

    let backup = Backup::new(&source_connection, &mut destination_connection)
        .context("failed to initialize SQLite Online Backup")?;
    run_backup_to_completion(&backup, BACKUP_BUSY_TIMEOUT)?;
    drop(backup);
    destination_connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .context("failed to finalize snapshot journal")?;
    Ok(())
}

fn run_backup_to_completion(backup: &Backup<'_, '_>, busy_timeout: Duration) -> anyhow::Result<()> {
    let mut busy_since = None;

    loop {
        match backup
            .step(BACKUP_PAGES_PER_STEP)
            .context("SQLite Online Backup failed")?
        {
            StepResult::Done => return Ok(()),
            StepResult::More => {
                busy_since = None;
                std::thread::sleep(BACKUP_STEP_PAUSE);
            }
            StepResult::Busy | StepResult::Locked => {
                let started = *busy_since.get_or_insert_with(Instant::now);
                let elapsed = started.elapsed();
                if elapsed >= busy_timeout {
                    bail!(
                        "SQLite Online Backup remained busy or locked for {} seconds",
                        busy_timeout.as_secs_f64()
                    );
                }
                std::thread::sleep(BACKUP_STEP_PAUSE.min(busy_timeout - elapsed));
            }
            _ => bail!("SQLite Online Backup returned an unexpected step result"),
        }
    }
}

fn validate_sqlite_integrity(path: &Path) -> anyhow::Result<()> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("failed to open snapshot {}", path.display()))?;
    connection.busy_timeout(SQLITE_BUSY_TIMEOUT)?;

    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .context("failed to run SQLite integrity_check")?;
    if integrity != "ok" {
        bail!("SQLite integrity_check failed: {integrity}");
    }

    let foreign_key_violation: Option<String> = connection
        .query_row(
            "SELECT printf('%s rowid=%s parent=%s', \"table\", rowid, parent) FROM pragma_foreign_key_check LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .context("failed to run SQLite foreign_key_check")?;
    if let Some(violation) = foreign_key_violation {
        bail!("SQLite foreign_key_check failed: {violation}");
    }
    Ok(())
}

fn supported_schema_version(directory: &Path) -> anyhow::Result<u32> {
    // Android's default temp directory may be inaccessible to the app.
    let scratch = Builder::new()
        .prefix("nitro-ark-schema-")
        .suffix(".sqlite")
        .tempfile_in(directory)
        .context("failed to create schema probe database")?;
    SqliteClient::open(scratch.path()).context("failed to initialize schema probe database")?;
    read_schema_version(scratch.path())
}

fn read_schema_version(path: &Path) -> anyhow::Result<u32> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to read schema version from {}", path.display()))?;
    let version: i64 = connection
        .query_row(
            "SELECT COALESCE(MAX(value), 0) FROM migrations",
            [],
            |row| row.get(0),
        )
        .context("snapshot does not contain a readable Bark migrations table")?;
    u32::try_from(version).context("snapshot schema version is invalid")
}

fn inspect_snapshot(
    snapshot_path: &Path,
    published_path: &Path,
) -> anyhow::Result<WalletSnapshotInfo> {
    let db = SqliteClient::open(snapshot_path).context("failed to inspect completed snapshot")?;
    let properties = crate::ffi::runtime::TOKIO_RT
        .block_on(db.read_properties())?
        .context("completed snapshot has no wallet properties")?;
    let schema_version = read_schema_version(snapshot_path)?;
    snapshot_info(snapshot_path, published_path, &properties, schema_version)
}

fn snapshot_info(
    content_path: &Path,
    reported_path: &Path,
    properties: &WalletProperties,
    schema_version: u32,
) -> anyhow::Result<WalletSnapshotInfo> {
    let metadata = content_path
        .metadata()
        .context("failed to read snapshot metadata")?;
    Ok(WalletSnapshotInfo {
        path: reported_path.to_string_lossy().into_owned(),
        size_bytes: metadata.len(),
        sha256: sha256_file(content_path)?,
        network: properties.network.to_string(),
        wallet_fingerprint: properties.fingerprint.to_string(),
        server_pubkey: properties.server_pubkey.map(|key| key.to_string()),
        mailbox_pubkey: properties.server_mailbox_pubkey.map(|key| key.to_string()),
        schema_version,
    })
}

fn sha256_file(path: &Path) -> anyhow::Result<String> {
    let file = File::open(path)
        .with_context(|| format!("failed to open snapshot for hashing: {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = sha256::Hash::engine();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.input(&buffer[..read]);
    }
    Ok(sha256::Hash::from_engine(hasher).to_string())
}

fn validate_expected_identity(
    properties: &WalletProperties,
    expected: Option<&WalletSnapshotExpectation>,
) -> anyhow::Result<()> {
    let Some(expected) = expected else {
        return Ok(());
    };

    if expected
        .network
        .as_ref()
        .is_some_and(|network| properties.network.to_string() != *network)
    {
        bail!(
            "Snapshot network mismatch: expected {}, found {}",
            expected.network.as_ref().unwrap(),
            properties.network
        );
    }
    if expected
        .wallet_fingerprint
        .as_ref()
        .is_some_and(|fingerprint| properties.fingerprint.to_string() != *fingerprint)
    {
        bail!(
            "Snapshot wallet fingerprint mismatch: expected {}, found {}",
            expected.wallet_fingerprint.as_ref().unwrap(),
            properties.fingerprint
        );
    }
    if let Some(server_pubkey) = &expected.server_pubkey {
        let actual = properties.server_pubkey.map(|key| key.to_string());
        if actual.as_deref() != Some(server_pubkey.as_str()) {
            bail!(
                "Snapshot server public key mismatch: expected {}, found {}",
                server_pubkey,
                actual.as_deref().unwrap_or("none")
            );
        }
    }
    Ok(())
}

use rusqlite::OptionalExtension;

fn parent_directory(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn same_file(left: &Path, right: &Path) -> anyhow::Result<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let a = left.metadata()?;
        let b = right.metadata()?;
        Ok(a.dev() == b.dev() && a.ino() == b.ino())
    }
    #[cfg(not(unix))]
    {
        Ok(left.canonicalize()? == right.canonicalize()?)
    }
}
#[cfg(test)]
mod tests {
    use bark::ark::bitcoin::bip32::Fingerprint;
    use bark::ark::bitcoin::Network;
    use tempfile::tempdir;

    use super::*;

    fn create_test_wallet_db(path: &Path) -> WalletProperties {
        let db = SqliteClient::open(path).unwrap();
        let properties = WalletProperties {
            network: Network::Signet,
            fingerprint: Fingerprint::from([1, 2, 3, 4]),
            server_pubkey: None,
            server_mailbox_pubkey: None,
        };
        crate::ffi::runtime::TOKIO_RT
            .block_on(async {
                db.init_wallet(&properties).await?;
                let _ = db.initialize_bdk_wallet().await?;
                anyhow::Ok(())
            })
            .unwrap();
        properties
    }

    #[test]
    fn creates_and_validates_consistent_snapshot() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("db.sqlite");
        let destination = dir.path().join("snapshot.sqlite");
        let properties = create_test_wallet_db(&source);

        let created = create_wallet_snapshot_blocking(&source, &destination).unwrap();
        assert_eq!(created.path, destination.to_string_lossy());
        assert_eq!(created.network, "signet");
        assert_eq!(
            created.wallet_fingerprint,
            properties.fingerprint.to_string()
        );
        assert_eq!(created.sha256.len(), 64);
        assert_eq!(created.size_bytes, destination.metadata().unwrap().len());

        let validated = validate_wallet_snapshot_blocking(
            &destination,
            Some(WalletSnapshotExpectation {
                network: Some("signet".to_string()),
                wallet_fingerprint: Some(properties.fingerprint.to_string()),
                server_pubkey: None,
            }),
        )
        .unwrap();
        assert_eq!(validated.sha256, created.sha256);
    }

    #[test]
    fn refuses_to_overwrite_existing_destination() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("db.sqlite");
        let destination = dir.path().join("snapshot.sqlite");
        create_test_wallet_db(&source);
        File::create(&destination).unwrap();

        let error = create_wallet_snapshot_blocking(&source, &destination).unwrap_err();
        assert!(error.to_string().contains("already exists"));
    }

    #[test]
    fn wal_snapshot_live_aliases_and_unchanged_input() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("db.sqlite");
        let snapshot = dir.path().join("snapshot.sqlite");
        create_test_wallet_db(&source);
        let writer = Connection::open(&source).unwrap();
        writer.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE backup_test(value); INSERT INTO backup_test VALUES(123);").unwrap();
        let live = register_live_database(&source).unwrap();
        let alias = dir.path().join("hardlink.sqlite");
        std::fs::hard_link(&source, &alias).unwrap();
        assert!(validate_wallet_snapshot_blocking(&source, None).is_err());
        assert!(validate_wallet_snapshot_blocking(&alias, None).is_err());
        assert!(create_wallet_snapshot_blocking(&source, &alias).is_err());
        let created = create_wallet_snapshot_blocking(&source, &snapshot).unwrap();
        let value: i64 = Connection::open(&snapshot)
            .unwrap()
            .query_row("SELECT value FROM backup_test", [], |r| r.get(0))
            .unwrap();
        assert_eq!(value, 123);
        let before = std::fs::read(&snapshot).unwrap();
        let info = validate_wallet_snapshot_blocking(&snapshot, None).unwrap();
        assert_eq!(info.sha256, created.sha256);
        assert_eq!(std::fs::read(&snapshot).unwrap(), before);
        drop(live);
    }

    #[test]
    fn validation_rejects_uncheckpointed_input_without_changing_any_file() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("db.sqlite");
        create_test_wallet_db(&source);
        let writer = Connection::open(&source).unwrap();
        writer.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE uncheckpointed(value); INSERT INTO uncheckpointed VALUES(123);").unwrap();
        let wal = dir.path().join("db.sqlite-wal");
        let before = std::fs::read(&source).unwrap();
        let wal_before = std::fs::read(&wal).unwrap();
        assert!(!wal_before.is_empty());
        let error = validate_wallet_snapshot_blocking(&source, None).unwrap_err();
        assert!(
            error.to_string().contains("self-contained snapshot"),
            "{error:#}"
        );
        assert_eq!(std::fs::read(&source).unwrap(), before);
        assert_eq!(std::fs::read(&wal).unwrap(), wal_before);
        writer
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .unwrap();
        assert!(validate_wallet_snapshot_blocking(&source, None).is_ok());
        let journal = dir.path().join("db.sqlite-journal");
        std::fs::write(&journal, b"nonempty journal requiring recovery").unwrap();
        assert!(validate_wallet_snapshot_blocking(&source, None)
            .unwrap_err()
            .to_string()
            .contains("self-contained snapshot"));
        assert_eq!(
            std::fs::read(&journal).unwrap(),
            b"nonempty journal requiring recovery"
        );
    }

    #[test]
    fn schema42_fixture_identity_and_unsupported_schema() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("db.sqlite");
        std::fs::write(
            &path,
            include_bytes!("../tests/fixtures/noah-v2-schema42.sqlite"),
        )
        .unwrap();
        let info = validate_wallet_snapshot_blocking(&path, None).unwrap();
        assert_eq!(info.schema_version, 42);
        assert_eq!(info.network, "regtest");
        assert_eq!(
            info.mailbox_pubkey.as_deref(),
            Some("02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5")
        );
        for expected in [
            WalletSnapshotExpectation {
                wallet_fingerprint: Some("00000000".into()),
                ..Default::default()
            },
            WalletSnapshotExpectation {
                server_pubkey: Some("wrong".into()),
                ..Default::default()
            },
        ] {
            assert!(validate_wallet_snapshot_blocking(&path, Some(expected)).is_err());
        }
        Connection::open(&path)
            .unwrap()
            .execute("INSERT INTO migrations(value) VALUES(9999)", [])
            .unwrap();
        assert!(validate_wallet_snapshot_blocking(&path, None)
            .unwrap_err()
            .to_string()
            .contains("newer"));
    }

    #[test]
    fn rejects_corrupt_snapshot_and_identity_mismatch() {
        let dir = tempdir().unwrap();
        let corrupt = dir.path().join("corrupt.sqlite");
        std::fs::write(&corrupt, b"not a sqlite database").unwrap();
        assert!(validate_wallet_snapshot_blocking(&corrupt, None).is_err());

        let source = dir.path().join("db.sqlite");
        let snapshot = dir.path().join("snapshot.sqlite");
        create_test_wallet_db(&source);
        create_wallet_snapshot_blocking(&source, &snapshot).unwrap();
        let error = validate_wallet_snapshot_blocking(
            &snapshot,
            Some(WalletSnapshotExpectation {
                network: Some("bitcoin".to_string()),
                ..Default::default()
            }),
        )
        .unwrap_err();
        assert!(error.to_string().contains("network mismatch"));
    }

    #[test]
    fn backup_times_out_during_lock_contention_and_recovers_after_unlock() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.sqlite");
        let destination = dir.path().join("destination.sqlite");
        let recovered_destination = dir.path().join("recovered.sqlite");

        let writer = Connection::open(&source).unwrap();
        writer
            .execute_batch(
                "CREATE TABLE test (value INTEGER NOT NULL);
                 INSERT INTO test VALUES (42);
                 BEGIN EXCLUSIVE;",
            )
            .unwrap();

        let source_connection = Connection::open_with_flags(
            &source,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .unwrap();
        source_connection.busy_timeout(BACKUP_STEP_PAUSE).unwrap();
        let mut destination_connection = Connection::open(&destination).unwrap();
        destination_connection
            .busy_timeout(BACKUP_STEP_PAUSE)
            .unwrap();
        let backup = Backup::new(&source_connection, &mut destination_connection).unwrap();
        let error = run_backup_to_completion(&backup, Duration::from_millis(50)).unwrap_err();
        assert!(error.to_string().contains("remained busy or locked"));
        drop(backup);
        drop(destination_connection);

        writer.execute_batch("ROLLBACK;").unwrap();

        sqlite_backup(&source, &recovered_destination).unwrap();
        let recovered = Connection::open(&recovered_destination).unwrap();
        let value: i64 = recovered
            .query_row("SELECT value FROM test", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, 42);
    }
}
