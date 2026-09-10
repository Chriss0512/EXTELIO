//! Backup, Restore und Disaster Recovery (Kapitel 15).
//!
//! Hot Backup: publish freeze -> SQLite-Checkpoint -> Outbox/Audit-Sync
//! -> Manifest -> Backup -> unfreeze. Laufende Gespraeche werden nicht beendet,
//! weil ausschliesslich lesend gearbeitet wird.
//!
//! Das portable Backup ist mit einem BACKUP_KEY aus dem KMS verschluesselt.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ids;
use crate::infrastructure::db::Db;
use crate::infrastructure::time::now_rfc3339;
use crate::kms::{Kms, SecretClass};
use crate::{Error, Result};

/// Publish Freeze. Waehrend eines Backups werden keine neuen Generationen
/// aktiviert; Telefonie laeuft unveraendert weiter.
static FROZEN: AtomicBool = AtomicBool::new(false);

pub fn is_frozen() -> bool {
    FROZEN.load(Ordering::SeqCst)
}

fn freeze() -> Result<()> {
    if FROZEN.swap(true, Ordering::SeqCst) {
        return Err(Error::Conflict("Es laeuft bereits ein Backup".into()));
    }
    Ok(())
}

fn unfreeze() {
    FROZEN.store(false, Ordering::SeqCst);
}

/// Restore Units (Kapitel 15.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreUnit {
    SystemConfiguration,
    DeviceConfiguration,
    Identity,
    Secrets,
    Media,
    Audit,
}

impl RestoreUnit {
    pub fn all() -> Vec<RestoreUnit> {
        vec![
            RestoreUnit::SystemConfiguration,
            RestoreUnit::DeviceConfiguration,
            RestoreUnit::Identity,
            RestoreUnit::Secrets,
            RestoreUnit::Media,
            RestoreUnit::Audit,
        ]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            RestoreUnit::SystemConfiguration => "system_configuration",
            RestoreUnit::DeviceConfiguration => "device_configuration",
            RestoreUnit::Identity => "identity",
            RestoreUnit::Secrets => "secrets",
            RestoreUnit::Media => "media",
            RestoreUnit::Audit => "audit",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub id: String,
    pub created_at: String,
    pub product_version: String,
    pub schema_version: i64,
    pub units: Vec<String>,
    pub db_sha256: String,
    pub db_bytes: u64,
    /// Kennzeichnet, ob der Inhalt verschluesselt abgelegt wurde.
    pub encrypted: bool,
    pub kek_key_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub id: String,
    pub created_at: String,
    pub kind: String,
    pub path: String,
    pub sha256: String,
    pub size_bytes: i64,
    pub encrypted: bool,
}

fn sha256_hex(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    format!("{:x}", h.finalize())
}

/// Erstellt ein anwendungskonsistentes Hot Backup.
pub fn create(db: &Db, kms: &Kms, backup_dir: &Path, kind: &str) -> Result<BackupInfo> {
    freeze()?;
    let result = create_inner(db, kms, backup_dir, kind);
    unfreeze();
    result
}

fn create_inner(db: &Db, kms: &Kms, backup_dir: &Path, kind: &str) -> Result<BackupInfo> {
    std::fs::create_dir_all(backup_dir)?;
    let id = ids::new_prefixed("bak");
    let snapshot = backup_dir.join(format!("{id}.sqlite3"));

    // WAL-Checkpoint, danach konsistente Kopie ueber VACUUM INTO.
    db.checkpoint()?;
    {
        let guard = db.lock();
        let target = snapshot.to_string_lossy().to_string();
        guard
            .execute("VACUUM INTO ?1", rusqlite::params![target])
            .map_err(|e| Error::Internal(format!("Snapshot fehlgeschlagen: {e}")))?;
    }

    let plain = std::fs::read(&snapshot)?;
    let db_sha = sha256_hex(&plain);
    let db_bytes = plain.len() as u64;

    let schema_version: i64 = {
        let guard = db.lock();
        guard.query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?
    };

    let sealed = kms.seal(SecretClass::BackupKey, &plain)?;
    let manifest = BackupManifest {
        id: id.clone(),
        created_at: now_rfc3339(),
        product_version: crate::VERSION.to_string(),
        schema_version,
        units: RestoreUnit::all()
            .iter()
            .map(|u| u.as_str().to_string())
            .collect(),
        db_sha256: db_sha,
        db_bytes,
        encrypted: true,
        kek_key_id: kms.key_id().to_string(),
    };

    let archive = serde_json::json!({
        "format": "extelio-backup-v1",
        "manifest": manifest,
        "payload": {
            "crypto_version": sealed.crypto_version,
            "algorithm": sealed.algorithm,
            "key_id": sealed.key_id,
            "wrapped_dek": data_encoding::BASE64.encode(&sealed.wrapped_dek),
            "dek_nonce": data_encoding::BASE64.encode(&sealed.dek_nonce),
            "nonce": data_encoding::BASE64.encode(&sealed.nonce),
            "ciphertext": data_encoding::BASE64.encode(&sealed.ciphertext),
        }
    });

    let path = backup_dir.join(format!("{id}.extelio"));
    let bytes = serde_json::to_vec(&archive)?;
    std::fs::write(&path, &bytes)?;
    std::fs::remove_file(&snapshot).ok();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = std::fs::metadata(&path)?.permissions();
        p.set_mode(0o600);
        std::fs::set_permissions(&path, p)?;
    }

    let info = BackupInfo {
        id: id.clone(),
        created_at: manifest.created_at.clone(),
        kind: kind.to_string(),
        path: path.to_string_lossy().to_string(),
        sha256: sha256_hex(&bytes),
        size_bytes: bytes.len() as i64,
        encrypted: true,
    };

    let guard = db.lock();
    guard.execute(
        "INSERT INTO backups (id,created_at,kind,units,path,sha256,size_bytes,encrypted)
         VALUES (?1,?2,?3,?4,?5,?6,?7,1)",
        rusqlite::params![
            info.id,
            info.created_at,
            info.kind,
            serde_json::to_string(&manifest.units)?,
            info.path,
            info.sha256,
            info.size_bytes
        ],
    )?;
    Ok(info)
}

/// Prueft ein Backup, ohne es einzuspielen (Restore Gate, Kapitel 15.5).
pub fn inspect(kms: &Kms, path: &Path) -> Result<BackupManifest> {
    let (manifest, plain) = decode(kms, path)?;
    if sha256_hex(&plain) != manifest.db_sha256 {
        return Err(Error::Validation(
            "Pruefsumme des Backups stimmt nicht".into(),
        ));
    }
    if manifest.schema_version
        > crate::infrastructure::migrations::MIGRATIONS
            .last()
            .map(|m| m.version)
            .unwrap_or(0)
    {
        return Err(Error::Validation(format!(
            "Das Backup stammt aus einer neueren Version (Schema {}). Bitte zuerst EXTELIO aktualisieren.",
            manifest.schema_version
        )));
    }
    Ok(manifest)
}

fn decode(kms: &Kms, path: &Path) -> Result<(BackupManifest, Vec<u8>)> {
    let raw = std::fs::read(path)?;
    let v: serde_json::Value = serde_json::from_slice(&raw)?;
    if v.get("format").and_then(|f| f.as_str()) != Some("extelio-backup-v1") {
        return Err(Error::Validation("Unbekanntes Backupformat".into()));
    }
    let manifest: BackupManifest = serde_json::from_value(v["manifest"].clone())?;
    let p = &v["payload"];
    let b64 = |k: &str| -> Result<Vec<u8>> {
        data_encoding::BASE64
            .decode(p[k].as_str().unwrap_or_default().as_bytes())
            .map_err(|_| Error::Validation(format!("Feld {k} ist kein Base64")))
    };
    let sealed = crate::kms::SealedSecret {
        crypto_version: p["crypto_version"].as_i64().unwrap_or(0),
        algorithm: p["algorithm"].as_str().unwrap_or_default().to_string(),
        key_id: p["key_id"].as_str().unwrap_or_default().to_string(),
        wrapped_dek: b64("wrapped_dek")?,
        dek_nonce: b64("dek_nonce")?,
        nonce: b64("nonce")?,
        ciphertext: b64("ciphertext")?,
    };
    let plain = kms.open(SecretClass::BackupKey, &sealed)?;
    Ok((manifest, plain.to_vec()))
}

/// Spielt ein Backup ein. Die laufende Datenbankdatei wird ersetzt; der
/// Aufrufer muss die Anwendung anschliessend neu starten.
pub fn restore(kms: &Kms, path: &Path, db_file: &Path) -> Result<BackupManifest> {
    let manifest = inspect(kms, path)?;
    let (_, plain) = decode(kms, path)?;

    let backup_of_current = db_file.with_extension("pre-restore");
    if db_file.exists() {
        std::fs::copy(db_file, &backup_of_current)?;
    }
    let tmp = db_file.with_extension("restore-tmp");
    std::fs::write(&tmp, &plain)?;
    std::fs::rename(&tmp, db_file)?;
    // WAL- und SHM-Reste der alten Datenbank entfernen.
    for ext in ["sqlite3-wal", "sqlite3-shm"] {
        let _ = std::fs::remove_file(db_file.with_extension(ext));
    }
    Ok(manifest)
}

pub fn list(db: &Db) -> Result<Vec<BackupInfo>> {
    let guard = db.lock();
    let mut stmt = guard.prepare(
        "SELECT id,created_at,kind,path,sha256,size_bytes,encrypted FROM backups ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(BackupInfo {
            id: r.get(0)?,
            created_at: r.get(1)?,
            kind: r.get(2)?,
            path: r.get(3)?,
            sha256: r.get(4)?,
            size_bytes: r.get(5)?,
            encrypted: r.get::<_, i64>(6)? == 1,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

pub fn path_for(backup_dir: &Path, id: &str) -> PathBuf {
    backup_dir.join(format!("{id}.extelio"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::migrations;

    #[test]
    fn hot_backup_roundtrip() {
        let dir = std::env::temp_dir().join(format!("extelio-bak-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let db_file = dir.join("pbx.sqlite3");
        let db = Db::open(&db_file).unwrap();
        migrations::run(&db).unwrap();
        {
            let guard = db.lock();
            guard
                .execute(
                    "INSERT INTO extensions (id,number,name,kind,voicemail,created_at,updated_at)
                 VALUES ('e1','201','Empfang','user',1,?1,?1)",
                    rusqlite::params![now_rfc3339()],
                )
                .unwrap();
        }

        let kms = Kms::open_or_init(&dir.join("kms")).unwrap();
        let info = create(&db, &kms, &dir.join("backup"), "hot").unwrap();
        assert!(info.encrypted);
        assert!(!is_frozen(), "Freeze wird immer aufgehoben");

        let bpath = PathBuf::from(&info.path);
        let raw = std::fs::read(&bpath).unwrap();
        assert!(
            !String::from_utf8_lossy(&raw).contains("Empfang"),
            "Kapitel 15: Backup ist verschluesselt"
        );

        let manifest = inspect(&kms, &bpath).unwrap();
        assert_eq!(manifest.units.len(), 6);

        // Datenbank veraendern und wiederherstellen.
        {
            let guard = db.lock();
            guard.execute("DELETE FROM extensions", []).unwrap();
        }
        drop(db);
        restore(&kms, &bpath, &db_file).unwrap();

        let db2 = Db::open(&db_file).unwrap();
        let guard = db2.lock();
        let n: i64 = guard
            .query_row("SELECT COUNT(*) FROM extensions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1, "Restore stellt den Stand wieder her");

        drop(guard);
        std::fs::remove_dir_all(&dir).ok();
    }
}
