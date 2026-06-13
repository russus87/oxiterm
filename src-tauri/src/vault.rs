//! Vault cifrato per le password salvate.
//!
//! Una master password sblocca il vault: da essa (con Argon2) si deriva una
//! chiave AES-256-GCM con cui cifriamo le singole password. Su disco finisce
//! solo testo cifrato; la chiave derivata vive solo in memoria a sblocco fatto.

use std::collections::HashMap;
use std::path::Path;

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, Nonce};
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};

const B64: base64::engine::general_purpose::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// Un valore cifrato (nonce + testo cifrato, entrambi in base64).
#[derive(Serialize, Deserialize, Default, Clone)]
struct Cifrato {
    nonce: String,
    dati: String,
}

/// Contenuto del file vault.
#[derive(Serialize, Deserialize, Default)]
struct FileVault {
    salt: String,
    sentinella: Cifrato,
    voci: HashMap<String, Cifrato>,
}

fn leggi(file: &Path) -> FileVault {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn scrivi(file: &Path, v: &FileVault) -> Result<(), String> {
    if let Some(d) = file.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(file, serde_json::to_string_pretty(v).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

/// Deriva la chiave a 32 byte dalla master password e dal salt.
fn deriva(master: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let mut chiave = [0u8; 32];
    argon2::Argon2::default()
        .hash_password_into(master.as_bytes(), salt, &mut chiave)
        .map_err(|e| e.to_string())?;
    Ok(chiave)
}

fn cifra(chiave: &[u8; 32], testo: &str) -> Result<Cifrato, String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(chiave));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let dati = cipher
        .encrypt(&nonce, testo.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(Cifrato {
        nonce: B64.encode(nonce),
        dati: B64.encode(dati),
    })
}

fn decifra(chiave: &[u8; 32], c: &Cifrato) -> Result<String, String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(chiave));
    let nonce = B64.decode(&c.nonce).map_err(|e| e.to_string())?;
    let dati = B64.decode(&c.dati).map_err(|e| e.to_string())?;
    let chiaro = cipher
        .decrypt(Nonce::from_slice(&nonce), dati.as_ref())
        .map_err(|_| "master password errata".to_string())?;
    String::from_utf8(chiaro).map_err(|e| e.to_string())
}

/// Esiste già un vault?
pub fn esiste(file: &Path) -> bool {
    file.exists()
}

/// Sblocca il vault (creandolo alla prima volta) e ritorna la chiave derivata.
pub fn sblocca(file: &Path, master: &str) -> Result<[u8; 32], String> {
    if !file.exists() {
        // Primo uso: crea salt + sentinella.
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        let chiave = deriva(master, &salt)?;
        let v = FileVault {
            salt: B64.encode(salt),
            sentinella: cifra(&chiave, "oxiterm")?,
            voci: HashMap::new(),
        };
        scrivi(file, &v)?;
        return Ok(chiave);
    }
    let v = leggi(file);
    let salt = B64.decode(&v.salt).map_err(|e| e.to_string())?;
    let chiave = deriva(master, &salt)?;
    // Verifica la master password decifrando la sentinella.
    if decifra(&chiave, &v.sentinella)? != "oxiterm" {
        return Err("master password errata".into());
    }
    Ok(chiave)
}

/// Salva (cifrata) la password di una sessione.
pub fn salva_password(file: &Path, chiave: &[u8; 32], id: &str, password: &str) -> Result<(), String> {
    let mut v = leggi(file);
    v.voci.insert(id.to_string(), cifra(chiave, password)?);
    scrivi(file, &v)
}

/// Legge (decifrata) la password di una sessione, se presente.
pub fn leggi_password(file: &Path, chiave: &[u8; 32], id: &str) -> Result<Option<String>, String> {
    let v = leggi(file);
    match v.voci.get(id) {
        Some(c) => Ok(Some(decifra(chiave, c)?)),
        None => Ok(None),
    }
}
