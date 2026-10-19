use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use log::{info, warn};

use bark::persist::sqlite::{SqliteClient, DEFAULT_DB_FILE};

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
