//! Operazioni sul filesystem remoto via SFTP (il pannello file di Oxiterm).
//! Tutte le funzioni lavorano su una `SftpSession` già aperta da `ssh::apri_sftp`.

use russh_sftp::client::SftpSession;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::model::VoceFile;

/// Cartella "home" di partenza (percorso assoluto di ".").
pub async fn home(sftp: &SftpSession) -> Result<String, String> {
    sftp.canonicalize(".").await.map_err(|e| e.to_string())
}

/// Elenca il contenuto di una cartella remota, cartelle prima e in ordine.
pub async fn lista(sftp: &SftpSession, percorso: &str) -> Result<Vec<VoceFile>, String> {
    let voci = sftp.read_dir(percorso).await.map_err(|e| e.to_string())?;
    let mut out: Vec<VoceFile> = voci
        .map(|e| {
            let meta = e.metadata();
            VoceFile {
                nome: e.file_name(),
                dir: meta.is_dir(),
                dimensione: meta.size.unwrap_or(0),
            }
        })
        .collect();
    out.sort_by(|a, b| b.dir.cmp(&a.dir).then(a.nome.to_lowercase().cmp(&b.nome.to_lowercase())));
    Ok(out)
}

/// Scarica un file remoto e lo scrive su disco locale.
pub async fn scarica(sftp: &SftpSession, remoto: &str, locale: &str) -> Result<(), String> {
    let mut f = sftp.open(remoto).await.map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).await.map_err(|e| e.to_string())?;
    std::fs::write(locale, buf).map_err(|e| e.to_string())
}

/// Carica un file locale verso un percorso remoto.
pub async fn carica(sftp: &SftpSession, locale: &str, remoto: &str) -> Result<(), String> {
    let dati = std::fs::read(locale).map_err(|e| e.to_string())?;
    let mut f = sftp.create(remoto).await.map_err(|e| e.to_string())?;
    f.write_all(&dati).await.map_err(|e| e.to_string())?;
    f.flush().await.map_err(|e| e.to_string())
}

/// Crea una cartella remota.
pub async fn crea_cartella(sftp: &SftpSession, percorso: &str) -> Result<(), String> {
    sftp.create_dir(percorso).await.map_err(|e| e.to_string())
}

/// Elimina un file o una cartella remota.
pub async fn elimina(sftp: &SftpSession, percorso: &str, dir: bool) -> Result<(), String> {
    if dir {
        sftp.remove_dir(percorso).await.map_err(|e| e.to_string())
    } else {
        sftp.remove_file(percorso).await.map_err(|e| e.to_string())
    }
}

/// Rinomina/sposta un file remoto.
pub async fn rinomina(sftp: &SftpSession, da: &str, a: &str) -> Result<(), String> {
    sftp.rename(da, a).await.map_err(|e| e.to_string())
}
