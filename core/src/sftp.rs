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

/// Carica un file locale segnalando l'avanzamento (byte scritti, totale).
pub async fn carica_progresso<F: Fn(u64, u64)>(
    sftp: &SftpSession,
    locale: &str,
    remoto: &str,
    prog: F,
) -> Result<(), String> {
    let dati = tokio::fs::read(locale).await.map_err(|e| e.to_string())?;
    let totale = dati.len() as u64;
    let mut f = sftp.create(remoto).await.map_err(|e| e.to_string())?;
    let mut scritti = 0u64;
    prog(0, totale);
    for chunk in dati.chunks(32 * 1024) {
        f.write_all(chunk).await.map_err(|e| e.to_string())?;
        scritti += chunk.len() as u64;
        prog(scritti, totale);
    }
    f.flush().await.map_err(|e| e.to_string())
}

/// Scarica un file remoto segnalando l'avanzamento (byte letti, totale).
pub async fn scarica_progresso<F: Fn(u64, u64)>(
    sftp: &SftpSession,
    remoto: &str,
    locale: &str,
    prog: F,
) -> Result<(), String> {
    let totale = sftp
        .metadata(remoto)
        .await
        .ok()
        .and_then(|m| m.size)
        .unwrap_or(0);
    let mut f = sftp.open(remoto).await.map_err(|e| e.to_string())?;
    let mut out = tokio::fs::File::create(locale).await.map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 32 * 1024];
    let mut letti = 0u64;
    prog(0, totale);
    loop {
        let n = f.read(&mut buf).await.map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).await.map_err(|e| e.to_string())?;
        letti += n as u64;
        prog(letti, totale.max(letti));
    }
    out.flush().await.map_err(|e| e.to_string())
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

/// Legge un file remoto come testo (per l'editor integrato).
pub async fn leggi_testo(sftp: &SftpSession, remoto: &str) -> Result<String, String> {
    let mut f = sftp.open(remoto).await.map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).await.map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}

/// Scrive testo in un file remoto (sovrascrive).
pub async fn scrivi_testo(sftp: &SftpSession, remoto: &str, contenuto: &str) -> Result<(), String> {
    let mut f = sftp.create(remoto).await.map_err(|e| e.to_string())?;
    f.write_all(contenuto.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    f.flush().await.map_err(|e| e.to_string())
}

/// Carica ricorsivamente una cartella locale su una remota.
pub async fn carica_cartella(
    sftp: &SftpSession,
    locale_dir: &str,
    remoto_dir: &str,
) -> Result<(), String> {
    let _ = sftp.create_dir(remoto_dir).await; // ok se esiste già
    let mut voci = tokio::fs::read_dir(locale_dir)
        .await
        .map_err(|e| e.to_string())?;
    while let Some(e) = voci.next_entry().await.map_err(|e| e.to_string())? {
        let nome = e.file_name().to_string_lossy().to_string();
        let percorso = e.path().to_string_lossy().to_string();
        let remoto = format!("{remoto_dir}/{nome}");
        let tipo = e.file_type().await.map_err(|e| e.to_string())?;
        if tipo.is_dir() {
            Box::pin(carica_cartella(sftp, &percorso, &remoto)).await?;
        } else {
            carica(sftp, &percorso, &remoto).await?;
        }
    }
    Ok(())
}

/// Scarica ricorsivamente una cartella remota su una locale.
pub async fn scarica_cartella(
    sftp: &SftpSession,
    remoto_dir: &str,
    locale_dir: &str,
) -> Result<(), String> {
    tokio::fs::create_dir_all(locale_dir)
        .await
        .map_err(|e| e.to_string())?;
    for v in lista(sftp, remoto_dir).await? {
        let remoto = format!("{remoto_dir}/{}", v.nome);
        let locale = format!("{locale_dir}/{}", v.nome);
        if v.dir {
            Box::pin(scarica_cartella(sftp, &remoto, &locale)).await?;
        } else {
            scarica(sftp, &remoto, &locale).await?;
        }
    }
    Ok(())
}
