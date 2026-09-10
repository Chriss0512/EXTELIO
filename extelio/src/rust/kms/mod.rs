//! Secret & Key Management (Kapitel 7).
//!
//! Envelope Encryption: Root KEK -> wrapped DEK pro Secret -> AES-256-GCM
//! Ciphertext. Crypto-Metadaten werden vollstaendig mitgefuehrt, damit
//! Crypto Agility (Kapitel 7.2) spaeter ohne Datenmigration moeglich bleibt.

pub mod broker;

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

use crate::infrastructure::db::Db;
use crate::infrastructure::time::now_rfc3339;
use crate::{Error, Result};

/// Aktive Crypto-Suite. Erhoehung nur mit Migrationspfad.
pub const CRYPTO_VERSION: i64 = 1;
pub const ALGORITHM: &str = "AES-256-GCM";

/// Secret Classes gemaess Kapitel 7.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretClass {
    AuthSecret,
    TotpSecret,
    SipCredential,
    TrunkCredential,
    DnsApiToken,
    AcmeSecret,
    ServiceToken,
    BackupKey,
    PrivateKey,
}

impl SecretClass {
    pub fn as_str(self) -> &'static str {
        match self {
            SecretClass::AuthSecret => "AUTH_SECRET",
            SecretClass::TotpSecret => "TOTP_SECRET",
            SecretClass::SipCredential => "SIP_CREDENTIAL",
            SecretClass::TrunkCredential => "TRUNK_CREDENTIAL",
            SecretClass::DnsApiToken => "DNS_API_TOKEN",
            SecretClass::AcmeSecret => "ACME_SECRET",
            SecretClass::ServiceToken => "SERVICE_TOKEN",
            SecretClass::BackupKey => "BACKUP_KEY",
            SecretClass::PrivateKey => "PRIVATE_KEY",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "AUTH_SECRET" => SecretClass::AuthSecret,
            "TOTP_SECRET" => SecretClass::TotpSecret,
            "SIP_CREDENTIAL" => SecretClass::SipCredential,
            "TRUNK_CREDENTIAL" => SecretClass::TrunkCredential,
            "DNS_API_TOKEN" => SecretClass::DnsApiToken,
            "ACME_SECRET" => SecretClass::AcmeSecret,
            "SERVICE_TOKEN" => SecretClass::ServiceToken,
            "BACKUP_KEY" => SecretClass::BackupKey,
            "PRIVATE_KEY" => SecretClass::PrivateKey,
            _ => return None,
        })
    }

    /// Darf der Klartext ueber die API sichtbar gemacht werden?
    /// TOTP-Seeds und private Schluessel niemals (Kapitel 14.5).
    pub fn revealable(self) -> bool {
        matches!(
            self,
            SecretClass::SipCredential | SecretClass::TrunkCredential | SecretClass::ServiceToken
        )
    }

    /// Rotationsintervall in Tagen; 0 bedeutet ereignisgesteuert.
    pub fn rotation_days(self) -> i64 {
        match self {
            SecretClass::ServiceToken => 90,
            SecretClass::SipCredential | SecretClass::TrunkCredential => 365,
            SecretClass::BackupKey => 730,
            _ => 0,
        }
    }

    /// Welche Prozess-Principals duerfen die Klasse beziehen (Kapitel 7.4)?
    pub fn allowed_principals(self) -> &'static [&'static str] {
        match self {
            SecretClass::AuthSecret | SecretClass::TotpSecret => &["pbx-web"],
            SecretClass::SipCredential | SecretClass::TrunkCredential => {
                &["pbx-core", "pbx-worker", "pbx-xml-adapter", "pbx-web"]
            }
            SecretClass::DnsApiToken | SecretClass::AcmeSecret => &["certbot", "pbx-worker"],
            SecretClass::ServiceToken => &["pbx-web", "pbx-worker"],
            SecretClass::BackupKey => &["pbx-worker", "pbx-web"],
            SecretClass::PrivateKey => &["pbx-web", "freeswitch"],
        }
    }
}

/// Herkunft des Root KEK (Kapitel 7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KekBackend {
    /// TPM-/HSM-gebunden.
    Hardware,
    /// Lokaler zufaelliger CSPRNG-KEK.
    Csprng,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KekFile {
    key_id: String,
    backend: KekBackend,
    created_at: String,
    /// Hex-kodiertes Schluesselmaterial. Datei liegt mit 0600 unter /data/kms.
    key_hex: String,
}

pub struct Kms {
    key_id: String,
    backend: KekBackend,
    kek: Zeroizing<[u8; 32]>,
    path: PathBuf,
}

/// Ein verschluesselter Datensatz inklusive Crypto-Metadaten (Kapitel 7.2).
#[derive(Debug, Clone)]
pub struct SealedSecret {
    pub crypto_version: i64,
    pub algorithm: String,
    pub key_id: String,
    pub wrapped_dek: Vec<u8>,
    pub dek_nonce: Vec<u8>,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

fn random_bytes(n: usize) -> Vec<u8> {
    let mut buf = vec![0u8; n];
    rand::rngs::OsRng.fill_bytes(&mut buf);
    buf
}

fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn hex_decode(s: &str) -> Result<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return Err(Error::Internal("hex: ungerade Laenge".into()));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| Error::Internal(format!("hex: {e}")))
        })
        .collect()
}

impl Kms {
    /// Laedt den Root KEK oder initialisiert ihn beim ersten Start.
    pub fn open_or_init(kms_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(kms_dir)?;
        let path = kms_dir.join("root.key");
        if path.exists() {
            let raw = std::fs::read_to_string(&path)?;
            let f: KekFile = serde_json::from_str(&raw)
                .map_err(|e| Error::Internal(format!("KMS-Datei unlesbar: {e}")))?;
            let bytes = hex_decode(&f.key_hex)?;
            if bytes.len() != 32 {
                return Err(Error::Internal("Root KEK hat nicht 256 Bit".into()));
            }
            let mut kek = [0u8; 32];
            kek.copy_from_slice(&bytes);
            return Ok(Self {
                key_id: f.key_id,
                backend: f.backend,
                kek: Zeroizing::new(kek),
                path,
            });
        }

        let backend = if hardware_backing_available() {
            KekBackend::Hardware
        } else {
            KekBackend::Csprng
        };
        let raw = random_bytes(32);
        let mut kek = [0u8; 32];
        kek.copy_from_slice(&raw);
        let f = KekFile {
            key_id: crate::domain::ids::new_prefixed("kek"),
            backend,
            created_at: now_rfc3339(),
            key_hex: hex_encode(&raw),
        };
        write_private(&path, &serde_json::to_vec_pretty(&f)?)?;
        Ok(Self {
            key_id: f.key_id,
            backend,
            kek: Zeroizing::new(kek),
            path,
        })
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }
    pub fn backend(&self) -> KekBackend {
        self.backend
    }

    fn cipher(key: &[u8; 32]) -> Aes256Gcm {
        Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key))
    }

    /// Verschluesselt Klartext mit einem frischen DEK (Envelope Encryption).
    /// Die Secret Class wird als Associated Data gebunden, damit ein Datensatz
    /// nicht in eine andere Klasse umgehaengt werden kann.
    pub fn seal(&self, class: SecretClass, plaintext: &[u8]) -> Result<SealedSecret> {
        let dek_raw = random_bytes(32);
        let mut dek = Zeroizing::new([0u8; 32]);
        dek.copy_from_slice(&dek_raw);

        let nonce_bytes = random_bytes(12);
        let ciphertext = Self::cipher(&dek)
            .encrypt(
                Nonce::from_slice(&nonce_bytes),
                Payload {
                    msg: plaintext,
                    aad: class.as_str().as_bytes(),
                },
            )
            .map_err(|_| Error::Internal("Verschluesselung fehlgeschlagen".into()))?;

        let dek_nonce = random_bytes(12);
        let wrapped_dek = Self::cipher(&self.kek)
            .encrypt(
                Nonce::from_slice(&dek_nonce),
                Payload {
                    msg: dek.as_ref(),
                    aad: self.key_id.as_bytes(),
                },
            )
            .map_err(|_| Error::Internal("DEK-Wrapping fehlgeschlagen".into()))?;

        Ok(SealedSecret {
            crypto_version: CRYPTO_VERSION,
            algorithm: ALGORITHM.into(),
            key_id: self.key_id.clone(),
            wrapped_dek,
            dek_nonce,
            nonce: nonce_bytes,
            ciphertext,
        })
    }

    pub fn open(&self, class: SecretClass, s: &SealedSecret) -> Result<Zeroizing<Vec<u8>>> {
        if s.crypto_version != CRYPTO_VERSION || s.algorithm != ALGORITHM {
            return Err(Error::Internal(format!(
                "Nicht unterstuetzte Crypto-Suite {} v{}",
                s.algorithm, s.crypto_version
            )));
        }
        if s.key_id != self.key_id {
            return Err(Error::Internal(
                "Secret wurde mit einem anderen Root KEK versiegelt".into(),
            ));
        }
        if s.wrapped_dek.is_empty() {
            return Err(Error::NotFound(
                "Secret wurde kryptografisch geloescht".into(),
            ));
        }

        let dek = Self::cipher(&self.kek)
            .decrypt(
                Nonce::from_slice(&s.dek_nonce),
                Payload {
                    msg: &s.wrapped_dek,
                    aad: self.key_id.as_bytes(),
                },
            )
            .map_err(|_| Error::Internal("DEK konnte nicht entpackt werden".into()))?;
        let mut dek_arr = Zeroizing::new([0u8; 32]);
        if dek.len() != 32 {
            return Err(Error::Internal("DEK hat falsche Laenge".into()));
        }
        dek_arr.copy_from_slice(&dek);

        let plain = Self::cipher(&dek_arr)
            .decrypt(
                Nonce::from_slice(&s.nonce),
                Payload {
                    msg: &s.ciphertext,
                    aad: class.as_str().as_bytes(),
                },
            )
            .map_err(|_| Error::Internal("Secret konnte nicht entschluesselt werden".into()))?;
        Ok(Zeroizing::new(plain))
    }

    /// Speichert ein Secret und liefert dessen ID.
    pub fn store(
        &self,
        db: &Db,
        class: SecretClass,
        label: &str,
        scope: &str,
        plaintext: &[u8],
    ) -> Result<String> {
        let sealed = self.seal(class, plaintext)?;
        let id = crate::domain::ids::new_prefixed("sec");
        let guard = db.lock();
        guard.execute(
            "INSERT INTO secrets
             (id,class,label,scope,crypto_version,algorithm,key_id,wrapped_dek,dek_nonce,nonce,ciphertext,created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            rusqlite::params![
                id, class.as_str(), label, scope, sealed.crypto_version, sealed.algorithm,
                sealed.key_id, sealed.wrapped_dek, sealed.dek_nonce, sealed.nonce,
                sealed.ciphertext, now_rfc3339()
            ],
        )?;
        Ok(id)
    }

    pub fn load(&self, db: &Db, id: &str) -> Result<Zeroizing<Vec<u8>>> {
        let (class, sealed) = {
            let guard = db.lock();
            guard.query_row(
                "SELECT class,crypto_version,algorithm,key_id,wrapped_dek,dek_nonce,nonce,ciphertext
                 FROM secrets WHERE id=?1 AND revoked_at IS NULL",
                rusqlite::params![id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        SealedSecret {
                            crypto_version: r.get(1)?,
                            algorithm: r.get(2)?,
                            key_id: r.get(3)?,
                            wrapped_dek: r.get(4)?,
                            dek_nonce: r.get(5)?,
                            nonce: r.get(6)?,
                            ciphertext: r.get(7)?,
                        },
                    ))
                },
            ).map_err(|_| Error::NotFound(format!("Secret {id} nicht gefunden")))?
        };
        let class = SecretClass::parse(&class)
            .ok_or_else(|| Error::Internal("Unbekannte Secret Class".into()))?;
        self.open(class, &sealed)
    }

    /// Crypto Erasure (Kapitel 7.5): Entfernen des wrapped DEK.
    pub fn crypto_erase(&self, db: &Db, id: &str) -> Result<()> {
        let guard = db.lock();
        guard.execute(
            "UPDATE secrets SET wrapped_dek=X'', revoked_at=?2 WHERE id=?1",
            rusqlite::params![id, now_rfc3339()],
        )?;
        Ok(())
    }

    /// KEK-Rotation durch Rewrap aller DEKs (Kapitel 7.5).
    pub fn rotate_kek(&mut self, db: &Db) -> Result<usize> {
        let raw = random_bytes(32);
        let mut new_kek = Zeroizing::new([0u8; 32]);
        new_kek.copy_from_slice(&raw);
        let new_key_id = crate::domain::ids::new_prefixed("kek");

        let rows: Vec<(String, Vec<u8>, Vec<u8>)> = {
            let guard = db.lock();
            let mut stmt = guard.prepare(
                "SELECT id,wrapped_dek,dek_nonce FROM secrets WHERE length(wrapped_dek)>0",
            )?;
            let it = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
            it.collect::<std::result::Result<_, _>>()?
        };

        let mut rewrapped = Vec::with_capacity(rows.len());
        for (id, wrapped, dek_nonce) in rows {
            let dek = Self::cipher(&self.kek)
                .decrypt(
                    Nonce::from_slice(&dek_nonce),
                    Payload {
                        msg: &wrapped,
                        aad: self.key_id.as_bytes(),
                    },
                )
                .map_err(|_| Error::Internal(format!("Rewrap fehlgeschlagen fuer {id}")))?;
            let nonce = random_bytes(12);
            let out = Self::cipher(&new_kek)
                .encrypt(
                    Nonce::from_slice(&nonce),
                    Payload {
                        msg: &dek,
                        aad: new_key_id.as_bytes(),
                    },
                )
                .map_err(|_| Error::Internal("Rewrap-Verschluesselung fehlgeschlagen".into()))?;
            rewrapped.push((id, out, nonce));
        }

        let count = rewrapped.len();
        {
            let mut guard = db.lock();
            let tx = guard.transaction()?;
            for (id, wrapped, nonce) in rewrapped {
                tx.execute(
                    "UPDATE secrets SET wrapped_dek=?2, dek_nonce=?3, key_id=?4, rotated_at=?5 WHERE id=?1",
                    rusqlite::params![id, wrapped, nonce, new_key_id, now_rfc3339()],
                )?;
            }
            tx.commit()?;
        }

        let f = KekFile {
            key_id: new_key_id.clone(),
            backend: self.backend,
            created_at: now_rfc3339(),
            key_hex: hex_encode(&raw),
        };
        write_private(&self.path, &serde_json::to_vec_pretty(&f)?)?;
        self.key_id = new_key_id;
        self.kek = new_kek;
        Ok(count)
    }
}

/// TPM-/HSM-Erkennung. Ohne nutzbare Hardware faellt EXTELIO bewusst auf einen
/// CSPRNG-KEK zurueck und verwendet keinen softwarebasierten Ersatz-Pepper
/// (Kapitel 6.2, 7.1).
pub fn hardware_backing_available() -> bool {
    if std::env::var("EXTELIO_FORCE_SOFTWARE_KEK").is_ok() {
        return false;
    }
    Path::new("/dev/tpmrm0").exists() && std::env::var("EXTELIO_TPM_ENABLED").is_ok()
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = std::fs::metadata(&tmp)?.permissions();
        p.set_mode(0o600);
        std::fs::set_permissions(&tmp, p)?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::migrations;

    fn tmpdir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("extelio-kms-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn seals_and_opens() {
        let dir = tmpdir("seal");
        let kms = Kms::open_or_init(&dir).unwrap();
        let sealed = kms.seal(SecretClass::SipCredential, b"s3cret").unwrap();
        let out = kms.open(SecretClass::SipCredential, &sealed).unwrap();
        assert_eq!(out.as_slice(), b"s3cret");
    }

    #[test]
    fn class_is_bound_as_associated_data() {
        let dir = tmpdir("aad");
        let kms = Kms::open_or_init(&dir).unwrap();
        let sealed = kms.seal(SecretClass::SipCredential, b"s3cret").unwrap();
        assert!(kms.open(SecretClass::TrunkCredential, &sealed).is_err());
    }

    #[test]
    fn store_load_erase_and_rotate() {
        let dir = tmpdir("store");
        let db = Db::open_memory().unwrap();
        migrations::run(&db).unwrap();
        let mut kms = Kms::open_or_init(&dir).unwrap();

        let id = kms
            .store(&db, SecretClass::TrunkCredential, "trunk", "system", b"pw")
            .unwrap();
        assert_eq!(kms.load(&db, &id).unwrap().as_slice(), b"pw");

        let n = kms.rotate_kek(&db).unwrap();
        assert_eq!(n, 1);
        assert_eq!(
            kms.load(&db, &id).unwrap().as_slice(),
            b"pw",
            "Rewrap erhaelt Klartext"
        );

        kms.crypto_erase(&db, &id).unwrap();
        assert!(
            kms.load(&db, &id).is_err(),
            "Crypto Erasure macht das Secret unlesbar"
        );
    }
}
