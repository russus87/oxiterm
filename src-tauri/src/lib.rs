//! Livello desktop di Oxiterm: espone i comandi richiamabili dal frontend e
//! fa da ponte tra la UI e il crate `oxiterm_core`.
//!
//! Lo stato condiviso (`Sessioni`) tiene una mappa id -> sessione attiva. Ogni
//! sessione conserva la connessione SSH, il canale per inviare input alla shell
//! e (a richiesta) un canale SFTP per il browser dei file.

use std::collections::HashMap;

use oxiterm_core::model::{Auth, Sessione, VoceFile};
use oxiterm_core::ssh::{ComandoSsh, Connessione, SftpSession};
use oxiterm_core::{sftp, storage};
use tauri::{Emitter, Manager, State};
use tokio::sync::mpsc;
use tokio::sync::Mutex;

/// Una sessione attualmente connessa.
struct SessioneAttiva {
    conn: Connessione,
    input: mpsc::Sender<ComandoSsh>,
    sftp: Option<SftpSession>,
}

/// Stato condiviso: tutte le sessioni connesse, indicizzate per id.
#[derive(Default)]
struct Sessioni(Mutex<HashMap<String, SessioneAttiva>>);

/// Percorso del file con la rubrica delle sessioni salvate.
fn file_sessioni(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("sessioni.json"))
}

// ---------------------------------------------------------------------------
// SSH: connessione e terminale
// ---------------------------------------------------------------------------

/// Apre una connessione SSH + shell e inizia a inoltrare l'output alla UI
/// tramite l'evento `ssh-dati-<id>`. Alla chiusura emette `ssh-chiuso-<id>`.
#[tauri::command]
async fn ssh_connetti(
    app: tauri::AppHandle,
    stato: State<'_, Sessioni>,
    id: String,
    host: String,
    porta: u16,
    utente: String,
    auth: Auth,
    colonne: u32,
    righe: u32,
) -> Result<(), String> {
    let conn = Connessione::connetti(&host, porta, &utente, auth).await?;
    let canale = conn.apri_shell(colonne, righe).await?;
    let input = canale.input.clone();
    let mut output = canale.output;

    // Task che inoltra l'output del server alla UI come eventi Tauri.
    let app2 = app.clone();
    let id2 = id.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(byte) = output.recv().await {
            let _ = app2.emit(&format!("ssh-dati-{id2}"), byte);
        }
        let _ = app2.emit(&format!("ssh-chiuso-{id2}"), ());
    });

    stato.0.lock().await.insert(
        id,
        SessioneAttiva {
            conn,
            input,
            sftp: None,
        },
    );
    Ok(())
}

/// Invia all'host i byte digitati dall'utente.
#[tauri::command]
async fn ssh_scrivi(stato: State<'_, Sessioni>, id: String, dati: String) -> Result<(), String> {
    let mappa = stato.0.lock().await;
    let s = mappa.get(&id).ok_or("sessione inesistente")?;
    s.input
        .send(ComandoSsh::Scrivi(dati.into_bytes()))
        .await
        .map_err(|_| "shell chiusa".to_string())
}

/// Comunica al server le nuove dimensioni del terminale.
#[tauri::command]
async fn ssh_ridimensiona(
    stato: State<'_, Sessioni>,
    id: String,
    colonne: u32,
    righe: u32,
) -> Result<(), String> {
    let mappa = stato.0.lock().await;
    if let Some(s) = mappa.get(&id) {
        let _ = s.input.send(ComandoSsh::Ridimensiona(colonne, righe)).await;
    }
    Ok(())
}

/// Chiude la sessione e libera le risorse.
#[tauri::command]
async fn ssh_disconnetti(stato: State<'_, Sessioni>, id: String) -> Result<(), String> {
    if let Some(s) = stato.0.lock().await.remove(&id) {
        let _ = s.input.send(ComandoSsh::Chiudi).await;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// SFTP: browser dei file remoti
// ---------------------------------------------------------------------------

/// Assicura che la sessione abbia un canale SFTP aperto, poi esegue `f`.
/// `f` riceve il riferimento al canale SFTP e restituisce un future.
async fn con_sftp<T, F, Fut>(stato: &State<'_, Sessioni>, id: &str, f: F) -> Result<T, String>
where
    F: FnOnce(SftpSession) -> Fut,
    Fut: std::future::Future<Output = (SftpSession, Result<T, String>)>,
{
    // Prende (o apre) il canale SFTP, lo estrae dalla mappa, lavora, lo rimette.
    let canale = {
        let mut mappa = stato.0.lock().await;
        let s = mappa.get_mut(id).ok_or("sessione inesistente")?;
        match s.sftp.take() {
            Some(c) => c,
            None => s.conn.apri_sftp().await?,
        }
    };
    let (canale, esito) = f(canale).await;
    if let Some(s) = stato.0.lock().await.get_mut(id) {
        s.sftp = Some(canale);
    }
    esito
}

/// Cartella iniziale (home) della sessione.
#[tauri::command]
async fn sftp_home(stato: State<'_, Sessioni>, id: String) -> Result<String, String> {
    con_sftp(&stato, &id, |c| async move {
        let r = sftp::home(&c).await;
        (c, r)
    })
    .await
}

/// Elenca una cartella remota.
#[tauri::command]
async fn sftp_lista(
    stato: State<'_, Sessioni>,
    id: String,
    percorso: String,
) -> Result<Vec<VoceFile>, String> {
    con_sftp(&stato, &id, |c| async move {
        let r = sftp::lista(&c, &percorso).await;
        (c, r)
    })
    .await
}

/// Scarica un file remoto su disco locale.
#[tauri::command]
async fn sftp_scarica(
    stato: State<'_, Sessioni>,
    id: String,
    remoto: String,
    locale: String,
) -> Result<(), String> {
    con_sftp(&stato, &id, |c| async move {
        let r = sftp::scarica(&c, &remoto, &locale).await;
        (c, r)
    })
    .await
}

/// Carica un file locale verso il server.
#[tauri::command]
async fn sftp_carica(
    stato: State<'_, Sessioni>,
    id: String,
    locale: String,
    remoto: String,
) -> Result<(), String> {
    con_sftp(&stato, &id, |c| async move {
        let r = sftp::carica(&c, &locale, &remoto).await;
        (c, r)
    })
    .await
}

/// Crea una cartella remota.
#[tauri::command]
async fn sftp_crea_cartella(
    stato: State<'_, Sessioni>,
    id: String,
    percorso: String,
) -> Result<(), String> {
    con_sftp(&stato, &id, |c| async move {
        let r = sftp::crea_cartella(&c, &percorso).await;
        (c, r)
    })
    .await
}

/// Elimina un file o una cartella remota.
#[tauri::command]
async fn sftp_elimina(
    stato: State<'_, Sessioni>,
    id: String,
    percorso: String,
    dir: bool,
) -> Result<(), String> {
    con_sftp(&stato, &id, |c| async move {
        let r = sftp::elimina(&c, &percorso, dir).await;
        (c, r)
    })
    .await
}

/// Rinomina/sposta un file remoto.
#[tauri::command]
async fn sftp_rinomina(
    stato: State<'_, Sessioni>,
    id: String,
    da: String,
    a: String,
) -> Result<(), String> {
    con_sftp(&stato, &id, |c| async move {
        let r = sftp::rinomina(&c, &da, &a).await;
        (c, r)
    })
    .await
}

// ---------------------------------------------------------------------------
// Session manager: rubrica delle sessioni salvate
// ---------------------------------------------------------------------------

/// Restituisce la rubrica delle sessioni salvate.
#[tauri::command]
fn lista_sessioni(app: tauri::AppHandle) -> Result<Vec<Sessione>, String> {
    Ok(storage::carica_sessioni(&file_sessioni(&app)?))
}

/// Inserisce o aggiorna una sessione nella rubrica (per id).
#[tauri::command]
fn salva_sessione(app: tauri::AppHandle, sessione: Sessione) -> Result<(), String> {
    let file = file_sessioni(&app)?;
    let mut tutte = storage::carica_sessioni(&file);
    match tutte.iter_mut().find(|s| s.id == sessione.id) {
        Some(esistente) => *esistente = sessione,
        None => tutte.push(sessione),
    }
    storage::salva_sessioni(&file, &tutte)
}

/// Rimuove una sessione dalla rubrica.
#[tauri::command]
fn elimina_sessione(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let file = file_sessioni(&app)?;
    let mut tutte = storage::carica_sessioni(&file);
    tutte.retain(|s| s.id != id);
    storage::salva_sessioni(&file, &tutte)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(Sessioni::default())
        .invoke_handler(tauri::generate_handler![
            ssh_connetti,
            ssh_scrivi,
            ssh_ridimensiona,
            ssh_disconnetti,
            sftp_home,
            sftp_lista,
            sftp_scarica,
            sftp_carica,
            sftp_crea_cartella,
            sftp_elimina,
            sftp_rinomina,
            lista_sessioni,
            salva_sessione,
            elimina_sessione,
        ])
        .run(tauri::generate_context!())
        .expect("errore durante l'avvio di Oxiterm");
}
