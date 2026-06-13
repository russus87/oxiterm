//! Gestione delle chiavi SSH locali: generazione, elenco e lettura della
//! chiave pubblica (per copiarla sul server con un "ssh-copy-id" via comando).

use std::path::PathBuf;

use russh::keys::ssh_key::{Algorithm, LineEnding, PrivateKey};
use serde::Serialize;

/// Informazioni su una chiave pubblica trovata in ~/.ssh.
#[derive(Serialize)]
pub struct ChiavePub {
    pub nome: String,
    pub percorso: String,
    pub contenuto: String,
}

/// Cartella home dell'utente (Unix o Windows).
fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_default()
}

/// Cartella ~/.ssh.
pub fn dir_ssh() -> PathBuf {
    home().join(".ssh")
}

/// Genera una nuova coppia di chiavi Ed25519 e la salva. Ritorna la pubblica.
pub fn genera(nome: &str, commento: &str) -> Result<String, String> {
    let dir = dir_ssh();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let percorso = dir.join(nome);

    let mut chiave =
        PrivateKey::random(&mut rand::rngs::OsRng, Algorithm::Ed25519).map_err(|e| e.to_string())?;
    chiave.set_comment(commento);

    let privata = chiave.to_openssh(LineEnding::LF).map_err(|e| e.to_string())?;
    std::fs::write(&percorso, privata.as_bytes()).map_err(|e| e.to_string())?;
    permessi_privati(&percorso);

    let pubblica = chiave
        .public_key()
        .to_openssh()
        .map_err(|e| e.to_string())?;
    std::fs::write(percorso.with_extension("pub"), &pubblica).map_err(|e| e.to_string())?;
    Ok(pubblica)
}

/// Elenca le chiavi pubbliche presenti in ~/.ssh.
pub fn lista() -> Vec<ChiavePub> {
    let mut out = Vec::new();
    if let Ok(voci) = std::fs::read_dir(dir_ssh()) {
        for v in voci.flatten() {
            let p = v.path();
            if p.extension().and_then(|e| e.to_str()) == Some("pub") {
                if let Ok(contenuto) = std::fs::read_to_string(&p) {
                    out.push(ChiavePub {
                        nome: p.file_name().unwrap_or_default().to_string_lossy().to_string(),
                        percorso: p.to_string_lossy().to_string(),
                        contenuto: contenuto.trim().to_string(),
                    });
                }
            }
        }
    }
    out
}

/// Imposta i permessi 600 sulla chiave privata (solo Unix).
#[cfg(unix)]
fn permessi_privati(percorso: &PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(percorso, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn permessi_privati(_percorso: &PathBuf) {}
