//! Livello desktop di Oxiterm: espone i comandi richiamabili dal frontend e
//! fa da ponte tra la UI e il crate `oxiterm_core`.
//!
//! Stato condiviso (`Sessioni`): mappa id -> sessione attiva. Ogni sessione ha
//! un canale d'ingresso (tasti/resize) e, se è SSH, la connessione (per SFTP e
//! tunnel). L'output di QUALSIASI terminale viene rimandato alla UI come eventi
//! `term-dati-<id>`; alla chiusura si emette `term-chiuso-<id>`.

use std::collections::HashMap;
use std::path::PathBuf;

use oxiterm_core::model::{Auth, Sessione, Snippet, VoceFile};
use oxiterm_core::ssh::{Connessione, ModoFiducia, SftpSession, StopTunnel};
use oxiterm_core::term::{Canale, ComandoTerm};
use oxiterm_core::{locale, seriale, sftp, telnet, storage};
use serde::Serialize;
use tauri::{Emitter, Manager, State};
use tokio::sync::mpsc;
use tokio::sync::Mutex;

/// Un tunnel attivo con la sua descrizione e il modo per fermarlo.
struct TunnelAttivo {
    id: String,
    descrizione: String,
    stop: StopTunnel,
}

/// Vista di un tunnel inviata al frontend.
#[derive(Serialize)]
struct TunnelView {
    id: String,
    descrizione: String,
}

/// Una sessione attualmente aperta (di qualsiasi tipo).
struct SessioneAttiva {
    input: mpsc::Sender<ComandoTerm>,
    /// Presente solo per le sessioni SSH (serve a SFTP e tunnel).
    ssh: Option<Connessione>,
    sftp: Option<SftpSession>,
    tunnel: Vec<TunnelAttivo>,
}

/// Stato condiviso: tutte le sessioni aperte, indicizzate per id.
#[derive(Default)]
struct Sessioni(Mutex<HashMap<String, SessioneAttiva>>);

/// File con la rubrica delle sessioni salvate.
fn file_sessioni(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("sessioni.json"))
}

/// File con le chiavi dei server conosciuti (known_hosts).
fn file_known_hosts(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("known_hosts.json"))
}

/// File con la libreria degli snippet.
fn file_snippet(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("snippet.json"))
}

/// Avvia l'inoltro dell'output del canale verso la UI e registra la sessione.
async fn registra(
    app: &tauri::AppHandle,
    stato: &State<'_, Sessioni>,
    id: String,
    canale: Canale,
    ssh: Option<Connessione>,
) {
    let input = canale.input.clone();
    let mut output = canale.output;

    let app2 = app.clone();
    let id2 = id.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(byte) = output.recv().await {
            let _ = app2.emit(&format!("term-dati-{id2}"), byte);
        }
        let _ = app2.emit(&format!("term-chiuso-{id2}"), ());
    });

    stato.0.lock().await.insert(
        id,
        SessioneAttiva {
            input,
            ssh,
            sftp: None,
            tunnel: Vec::new(),
        },
    );
}

// ---------------------------------------------------------------------------
// Apertura sessioni (SSH, locale, Telnet, seriale)
// ---------------------------------------------------------------------------

/// Apre una connessione SSH + shell.
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
    modo: Option<String>,
) -> Result<(), String> {
    let known = file_known_hosts(&app)?;
    let modo = match modo.as_deref() {
        Some("accetta") => ModoFiducia::AccettaNuova,
        Some("sostituisci") => ModoFiducia::Sostituisci,
        _ => ModoFiducia::Normale,
    };
    let conn = Connessione::connetti(&host, porta, &utente, auth, known, modo).await?;
    let canale = conn.apri_shell(colonne, righe).await?;
    registra(&app, &stato, id, canale, Some(conn)).await;
    Ok(())
}

/// Apre un terminale locale (shell di sistema).
#[tauri::command]
async fn apri_locale(
    app: tauri::AppHandle,
    stato: State<'_, Sessioni>,
    id: String,
    shell: Option<String>,
    colonne: u16,
    righe: u16,
) -> Result<(), String> {
    let canale = locale::apri(shell, colonne, righe)?;
    registra(&app, &stato, id, canale, None).await;
    Ok(())
}

/// Apre una connessione Telnet.
#[tauri::command]
async fn apri_telnet(
    app: tauri::AppHandle,
    stato: State<'_, Sessioni>,
    id: String,
    host: String,
    porta: u16,
) -> Result<(), String> {
    let canale = telnet::apri(&host, porta).await?;
    registra(&app, &stato, id, canale, None).await;
    Ok(())
}

/// Apre una console seriale.
#[tauri::command]
async fn apri_seriale(
    app: tauri::AppHandle,
    stato: State<'_, Sessioni>,
    id: String,
    porta: String,
    baud: u32,
) -> Result<(), String> {
    let canale = seriale::apri(&porta, baud)?;
    registra(&app, &stato, id, canale, None).await;
    Ok(())
}

/// Elenca le porte seriali disponibili.
#[tauri::command]
fn porte_seriali() -> Vec<String> {
    seriale::porte()
}

// ---------------------------------------------------------------------------
// Terminale: input / resize / chiusura
// ---------------------------------------------------------------------------

#[tauri::command]
async fn term_scrivi(stato: State<'_, Sessioni>, id: String, dati: String) -> Result<(), String> {
    let mappa = stato.0.lock().await;
    let s = mappa.get(&id).ok_or("sessione inesistente")?;
    s.input
        .send(ComandoTerm::Scrivi(dati.into_bytes()))
        .await
        .map_err(|_| "terminale chiuso".to_string())
}

#[tauri::command]
async fn term_ridimensiona(
    stato: State<'_, Sessioni>,
    id: String,
    colonne: u32,
    righe: u32,
) -> Result<(), String> {
    let mappa = stato.0.lock().await;
    if let Some(s) = mappa.get(&id) {
        let _ = s.input.send(ComandoTerm::Ridimensiona(colonne, righe)).await;
    }
    Ok(())
}

#[tauri::command]
async fn term_chiudi(stato: State<'_, Sessioni>, id: String) -> Result<(), String> {
    if let Some(s) = stato.0.lock().await.remove(&id) {
        let _ = s.input.send(ComandoTerm::Chiudi).await;
        for t in &s.tunnel {
            t.stop.ferma().await;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tunnel SSH (port forwarding)
// ---------------------------------------------------------------------------

#[tauri::command]
async fn tunnel_locale(
    stato: State<'_, Sessioni>,
    id: String,
    porta_locale: u16,
    host_remoto: String,
    porta_remota: u16,
) -> Result<String, String> {
    let mut mappa = stato.0.lock().await;
    let s = mappa.get_mut(&id).ok_or("sessione inesistente")?;
    let conn = s.ssh.as_ref().ok_or("i tunnel sono disponibili solo via SSH")?;
    let stop = conn
        .tunnel_locale(porta_locale, host_remoto.clone(), porta_remota)
        .await?;
    let tid = format!("L{porta_locale}");
    s.tunnel.push(TunnelAttivo {
        id: tid.clone(),
        descrizione: format!("locale :{porta_locale} → {host_remoto}:{porta_remota}"),
        stop,
    });
    Ok(tid)
}

#[tauri::command]
async fn tunnel_socks(
    stato: State<'_, Sessioni>,
    id: String,
    porta_locale: u16,
) -> Result<String, String> {
    let mut mappa = stato.0.lock().await;
    let s = mappa.get_mut(&id).ok_or("sessione inesistente")?;
    let conn = s.ssh.as_ref().ok_or("i tunnel sono disponibili solo via SSH")?;
    let stop = conn.tunnel_socks(porta_locale).await?;
    let tid = format!("D{porta_locale}");
    s.tunnel.push(TunnelAttivo {
        id: tid.clone(),
        descrizione: format!("SOCKS5 dinamico :{porta_locale}"),
        stop,
    });
    Ok(tid)
}

#[tauri::command]
async fn tunnel_remoto(
    stato: State<'_, Sessioni>,
    id: String,
    porta_remota: u16,
    host_locale: String,
    porta_locale: u16,
) -> Result<String, String> {
    let mut mappa = stato.0.lock().await;
    let s = mappa.get_mut(&id).ok_or("sessione inesistente")?;
    let conn = s.ssh.as_ref().ok_or("i tunnel sono disponibili solo via SSH")?;
    let stop = conn
        .tunnel_remoto(porta_remota, host_locale.clone(), porta_locale)
        .await?;
    let tid = format!("R{porta_remota}");
    s.tunnel.push(TunnelAttivo {
        id: tid.clone(),
        descrizione: format!("remoto :{porta_remota} → {host_locale}:{porta_locale}"),
        stop,
    });
    Ok(tid)
}

#[tauri::command]
async fn lista_tunnel(stato: State<'_, Sessioni>, id: String) -> Result<Vec<TunnelView>, String> {
    let mappa = stato.0.lock().await;
    let s = mappa.get(&id).ok_or("sessione inesistente")?;
    Ok(s.tunnel
        .iter()
        .map(|t| TunnelView {
            id: t.id.clone(),
            descrizione: t.descrizione.clone(),
        })
        .collect())
}

#[tauri::command]
async fn ferma_tunnel(
    stato: State<'_, Sessioni>,
    id: String,
    tunnel_id: String,
) -> Result<(), String> {
    let mut mappa = stato.0.lock().await;
    let s = mappa.get_mut(&id).ok_or("sessione inesistente")?;
    if let Some(pos) = s.tunnel.iter().position(|t| t.id == tunnel_id) {
        let t = s.tunnel.remove(pos);
        t.stop.ferma().await;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// SFTP: browser dei file remoti
// ---------------------------------------------------------------------------

/// Assicura che la sessione abbia un canale SFTP aperto, poi esegue `f`.
async fn con_sftp<T, F, Fut>(stato: &Sessioni, id: &str, f: F) -> Result<T, String>
where
    F: FnOnce(SftpSession) -> Fut,
    Fut: std::future::Future<Output = (SftpSession, Result<T, String>)>,
{
    let canale = {
        let mut mappa = stato.0.lock().await;
        let s = mappa.get_mut(id).ok_or("sessione inesistente")?;
        let conn = s.ssh.as_ref().ok_or("SFTP disponibile solo via SSH")?;
        match s.sftp.take() {
            Some(c) => c,
            None => conn.apri_sftp().await?,
        }
    };
    let (canale, esito) = f(canale).await;
    if let Some(s) = stato.0.lock().await.get_mut(id) {
        s.sftp = Some(canale);
    }
    esito
}

#[tauri::command]
async fn sftp_home(stato: State<'_, Sessioni>, id: String) -> Result<String, String> {
    con_sftp(&stato, &id, |c| async move {
        let r = sftp::home(&c).await;
        (c, r)
    })
    .await
}

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

/// Scarica un file remoto in una cartella temporanea, restituisce il percorso
/// locale (che il frontend aprirà con l'editor di sistema) e avvia un piccolo
/// task che lo ricarica sul server ogni volta che il file locale cambia.
#[tauri::command]
async fn sftp_apri_editor(
    app: tauri::AppHandle,
    stato: State<'_, Sessioni>,
    id: String,
    remoto: String,
) -> Result<String, String> {
    let nome = remoto
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("file")
        .to_string();
    let dir = std::env::temp_dir().join("oxiterm");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let locale = dir.join(&nome).to_string_lossy().to_string();

    // Scarica subito.
    {
        let rem = remoto.clone();
        let loc = locale.clone();
        con_sftp(&stato, &id, move |c| async move {
            let r = sftp::scarica(&c, &rem, &loc).await;
            (c, r)
        })
        .await?;
    }

    // Auto-salvataggio: ogni secondo controlla la data di modifica e, se cambia,
    // ricarica il file sul server. Si ferma se il file sparisce o la sessione chiude.
    let app2 = app.clone();
    let id2 = id.clone();
    let rem2 = remoto.clone();
    let loc2 = locale.clone();
    tauri::async_runtime::spawn(async move {
        let mtime = |p: &str| std::fs::metadata(p).and_then(|m| m.modified()).ok();
        let mut ultimo = mtime(&loc2);
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let ora = mtime(&loc2);
            if ora.is_none() {
                break;
            }
            if ora != ultimo {
                ultimo = ora;
                let stato = app2.state::<Sessioni>();
                let rem = rem2.clone();
                let loc = loc2.clone();
                let esito = con_sftp(&stato, &id2, move |c| async move {
                    let r = sftp::carica(&c, &loc, &rem).await;
                    (c, r)
                })
                .await;
                if esito.is_err() {
                    break;
                }
            }
        }
    });

    Ok(locale)
}

// ---------------------------------------------------------------------------
// Session manager: rubrica delle sessioni salvate
// ---------------------------------------------------------------------------

#[tauri::command]
fn lista_sessioni(app: tauri::AppHandle) -> Result<Vec<Sessione>, String> {
    Ok(storage::carica_sessioni(&file_sessioni(&app)?))
}

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

#[tauri::command]
fn elimina_sessione(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let file = file_sessioni(&app)?;
    let mut tutte = storage::carica_sessioni(&file);
    tutte.retain(|s| s.id != id);
    storage::salva_sessioni(&file, &tutte)
}

/// Esporta la rubrica delle sessioni in un file JSON scelto dall'utente.
#[tauri::command]
fn esporta_rubrica(app: tauri::AppHandle, percorso: String) -> Result<(), String> {
    let tutte = storage::carica_sessioni(&file_sessioni(&app)?);
    let testo = serde_json::to_string_pretty(&tutte).map_err(|e| e.to_string())?;
    std::fs::write(&percorso, testo).map_err(|e| e.to_string())
}

/// Importa sessioni da un file JSON e le unisce alla rubrica (per id).
#[tauri::command]
fn importa_rubrica(app: tauri::AppHandle, percorso: String) -> Result<usize, String> {
    let testo = std::fs::read_to_string(&percorso).map_err(|e| e.to_string())?;
    let nuove: Vec<Sessione> = serde_json::from_str(&testo).map_err(|e| e.to_string())?;
    let file = file_sessioni(&app)?;
    let mut tutte = storage::carica_sessioni(&file);
    let mut aggiunte = 0;
    for n in nuove {
        match tutte.iter_mut().find(|s| s.id == n.id) {
            Some(esistente) => *esistente = n,
            None => {
                tutte.push(n);
                aggiunte += 1;
            }
        }
    }
    storage::salva_sessioni(&file, &tutte)?;
    Ok(aggiunte)
}

// ---------------------------------------------------------------------------
// Snippet / macro
// ---------------------------------------------------------------------------

#[tauri::command]
fn lista_snippet(app: tauri::AppHandle) -> Result<Vec<Snippet>, String> {
    Ok(storage::carica_snippet(&file_snippet(&app)?))
}

#[tauri::command]
fn salva_snippet(app: tauri::AppHandle, snippet: Snippet) -> Result<(), String> {
    let file = file_snippet(&app)?;
    let mut tutti = storage::carica_snippet(&file);
    match tutti.iter_mut().find(|s| s.id == snippet.id) {
        Some(esistente) => *esistente = snippet,
        None => tutti.push(snippet),
    }
    storage::salva_snippet(&file, &tutti)
}

#[tauri::command]
fn elimina_snippet(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let file = file_snippet(&app)?;
    let mut tutti = storage::carica_snippet(&file);
    tutti.retain(|s| s.id != id);
    storage::salva_snippet(&file, &tutti)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Sessioni::default())
        .invoke_handler(tauri::generate_handler![
            ssh_connetti,
            apri_locale,
            apri_telnet,
            apri_seriale,
            porte_seriali,
            term_scrivi,
            term_ridimensiona,
            term_chiudi,
            tunnel_locale,
            tunnel_socks,
            tunnel_remoto,
            lista_tunnel,
            ferma_tunnel,
            sftp_home,
            sftp_lista,
            sftp_scarica,
            sftp_carica,
            sftp_crea_cartella,
            sftp_elimina,
            sftp_rinomina,
            sftp_apri_editor,
            lista_sessioni,
            salva_sessione,
            elimina_sessione,
            esporta_rubrica,
            importa_rubrica,
            lista_snippet,
            salva_snippet,
            elimina_snippet,
        ])
        .run(tauri::generate_context!())
        .expect("errore durante l'avvio di Oxiterm");
}
