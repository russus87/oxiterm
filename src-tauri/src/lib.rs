//! Livello desktop di Oxiterm: espone i comandi richiamabili dal frontend e
//! fa da ponte tra la UI e il crate `oxiterm_core`.
//!
//! Stato condiviso (`Sessioni`): mappa id -> sessione attiva. Ogni sessione ha
//! un canale d'ingresso (tasti/resize) e, se è SSH, la connessione (per SFTP e
//! tunnel). L'output di QUALSIASI terminale viene rimandato alla UI come eventi
//! `term-dati-<id>`; alla chiusura si emette `term-chiuso-<id>`.

mod git_sync;
mod vault;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use oxiterm_core::model::{Auth, JumpConfig, Sessione, Snippet, VoceFile};
use oxiterm_core::ssh::{Connessione, ModoFiducia, SftpSession, StopTunnel};
use oxiterm_core::term::{Canale, ComandoTerm};
use oxiterm_core::vnc::{self, ComandoVnc};
use oxiterm_core::{locale, seriale, sftp, telnet, storage};
use base64::Engine;
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

/// File di log della sessione (se attivo). Condiviso col task di inoltro output.
type LogSessione = std::sync::Arc<std::sync::Mutex<Option<std::fs::File>>>;
/// Registrazione asciicast (file + istante di inizio).
type RegSessione = std::sync::Arc<std::sync::Mutex<Option<(std::fs::File, Instant)>>>;

/// Una sessione attualmente aperta (di qualsiasi tipo).
struct SessioneAttiva {
    input: mpsc::Sender<ComandoTerm>,
    /// Presente solo per le sessioni SSH (serve a SFTP e tunnel).
    ssh: Option<Connessione>,
    sftp: Option<SftpSession>,
    tunnel: Vec<TunnelAttivo>,
    log: LogSessione,
    reg: RegSessione,
}

/// Avanzamento di un trasferimento SFTP, inviato alla UI.
#[derive(Clone, Serialize)]
struct ProgressoSftp {
    id: String,
    fatti: u64,
    totale: u64,
}

/// Stato condiviso: tutte le sessioni aperte, indicizzate per id.
#[derive(Default)]
struct Sessioni(Mutex<HashMap<String, SessioneAttiva>>);

/// Stato delle sessioni VNC aperte (id -> canale d'ingresso).
#[derive(Default)]
struct StatoVnc(Mutex<HashMap<String, mpsc::Sender<ComandoVnc>>>);

/// Un rettangolo di schermo VNC inviato alla UI (rgba in base64).
#[derive(Clone, Serialize)]
struct FrameVncView {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    dati: String,
    resize: Option<(u16, u16)>,
    cursore: Option<(u16, u16)>,
}

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

/// File del vault cifrato.
fn file_vault(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("vault.json"))
}

/// Chiave del vault in memoria (presente solo dopo lo sblocco).
#[derive(Default)]
struct StatoVault(std::sync::Mutex<Option<[u8; 32]>>);

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
    let log: LogSessione = std::sync::Arc::new(std::sync::Mutex::new(None));
    let reg: RegSessione = std::sync::Arc::new(std::sync::Mutex::new(None));

    let app2 = app.clone();
    let id2 = id.clone();
    let log2 = log.clone();
    let reg2 = reg.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(byte) = output.recv().await {
            // Se il log è attivo, scrivi l'output grezzo su file.
            if let Ok(mut g) = log2.lock() {
                if let Some(f) = g.as_mut() {
                    use std::io::Write;
                    let _ = f.write_all(&byte);
                }
            }
            // Se è in corso una registrazione, scrivi un evento asciicast.
            if let Ok(mut g) = reg2.lock() {
                if let Some((f, inizio)) = g.as_mut() {
                    use std::io::Write;
                    let t = inizio.elapsed().as_secs_f64();
                    let testo = String::from_utf8_lossy(&byte);
                    if let Ok(linea) = serde_json::to_string(&(t, "o", testo.as_ref())) {
                        let _ = writeln!(f, "{linea}");
                    }
                }
            }
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
            log,
            reg,
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
    jump: Option<JumpConfig>,
) -> Result<(), String> {
    let known = file_known_hosts(&app)?;
    let modo = match modo.as_deref() {
        Some("accetta") => ModoFiducia::AccettaNuova,
        Some("sostituisci") => ModoFiducia::Sostituisci,
        _ => ModoFiducia::Normale,
    };
    let conn = Connessione::connetti(&host, porta, &utente, auth, known, modo, jump).await?;
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
// VNC (sperimentale)
// ---------------------------------------------------------------------------

/// Apre una sessione VNC e inoltra i frame alla UI (`vnc-frame-<id>`).
#[tauri::command]
async fn apri_vnc(
    app: tauri::AppHandle,
    stato: State<'_, StatoVnc>,
    id: String,
    host: String,
    porta: u16,
    password: Option<String>,
) -> Result<(), String> {
    let canale = vnc::apri(&host, porta, password)?;
    let input = canale.input.clone();
    let mut frame = canale.frame;

    let app2 = app.clone();
    let id2 = id.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(f) = frame.recv().await {
            let dati = base64::engine::general_purpose::STANDARD.encode(&f.rgba);
            let _ = app2.emit(
                &format!("vnc-frame-{id2}"),
                FrameVncView {
                    x: f.x,
                    y: f.y,
                    w: f.w,
                    h: f.h,
                    dati,
                    resize: f.resize,
                    cursore: f.cursore,
                },
            );
        }
        let _ = app2.emit(&format!("vnc-chiuso-{id2}"), ());
    });

    stato.0.lock().await.insert(id, input);
    Ok(())
}

#[tauri::command]
async fn vnc_mouse(
    stato: State<'_, StatoVnc>,
    id: String,
    x: u16,
    y: u16,
    bottoni: u8,
) -> Result<(), String> {
    if let Some(tx) = stato.0.lock().await.get(&id) {
        let _ = tx.send(ComandoVnc::Mouse { x, y, bottoni }).await;
    }
    Ok(())
}

#[tauri::command]
async fn vnc_tasto(
    stato: State<'_, StatoVnc>,
    id: String,
    giu: bool,
    key: u32,
) -> Result<(), String> {
    if let Some(tx) = stato.0.lock().await.get(&id) {
        let _ = tx.send(ComandoVnc::Tasto { giu, key }).await;
    }
    Ok(())
}

#[tauri::command]
async fn vnc_chiudi(stato: State<'_, StatoVnc>, id: String) -> Result<(), String> {
    if let Some(tx) = stato.0.lock().await.remove(&id) {
        let _ = tx.send(ComandoVnc::Chiudi).await;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Strumenti di rete (locali) e monitor server
// ---------------------------------------------------------------------------

#[tauri::command]
async fn net_porta(host: String, porta: u16) -> bool {
    oxiterm_core::net::porta_aperta(&host, porta).await
}

#[tauri::command]
fn net_ping(host: String) -> Result<String, String> {
    oxiterm_core::net::ping(&host)
}

#[tauri::command]
fn net_traceroute(host: String) -> Result<String, String> {
    oxiterm_core::net::traceroute(&host)
}

#[tauri::command]
fn net_wol(mac: String) -> Result<(), String> {
    oxiterm_core::net::wake_on_lan(&mac)
}

/// Snapshot dello stato del server (uptime, memoria, disco, processi) via SSH.
#[tauri::command]
async fn server_stato(stato: State<'_, Sessioni>, id: String) -> Result<String, String> {
    let mappa = stato.0.lock().await;
    let s = mappa.get(&id).ok_or("sessione inesistente")?;
    let conn = s.ssh.as_ref().ok_or("disponibile solo per sessioni SSH")?;
    conn.esegui(
        "export LANG=C; echo '== UPTIME =='; uptime; \
         echo; echo '== MEMORIA =='; (free -h 2>/dev/null || vm_stat 2>/dev/null); \
         echo; echo '== DISCO =='; df -h / 2>/dev/null; \
         echo; echo '== PROCESSI =='; (top -bn1 2>/dev/null | head -12 || ps aux | head -12)",
    )
    .await
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

/// Avvia la registrazione su file dell'output della sessione.
#[tauri::command]
async fn term_log_avvia(
    stato: State<'_, Sessioni>,
    id: String,
    percorso: String,
) -> Result<(), String> {
    let mappa = stato.0.lock().await;
    let s = mappa.get(&id).ok_or("sessione inesistente")?;
    let f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&percorso)
        .map_err(|e| e.to_string())?;
    *s.log.lock().unwrap() = Some(f);
    Ok(())
}

/// Ferma la registrazione su file.
#[tauri::command]
async fn term_log_ferma(stato: State<'_, Sessioni>, id: String) -> Result<(), String> {
    let mappa = stato.0.lock().await;
    if let Some(s) = mappa.get(&id) {
        *s.log.lock().unwrap() = None;
    }
    Ok(())
}

/// Avvia la registrazione asciicast della sessione (replay successivo).
#[tauri::command]
async fn term_rec_avvia(
    stato: State<'_, Sessioni>,
    id: String,
    percorso: String,
) -> Result<(), String> {
    let mappa = stato.0.lock().await;
    let s = mappa.get(&id).ok_or("sessione inesistente")?;
    let mut f = std::fs::File::create(&percorso).map_err(|e| e.to_string())?;
    use std::io::Write;
    // Intestazione asciicast v2 (dimensioni indicative).
    writeln!(f, "{{\"version\":2,\"width\":120,\"height\":32}}").map_err(|e| e.to_string())?;
    *s.reg.lock().unwrap() = Some((f, Instant::now()));
    Ok(())
}

/// Ferma la registrazione.
#[tauri::command]
async fn term_rec_ferma(stato: State<'_, Sessioni>, id: String) -> Result<(), String> {
    let mappa = stato.0.lock().await;
    if let Some(s) = mappa.get(&id) {
        *s.reg.lock().unwrap() = None;
    }
    Ok(())
}

/// Legge un file locale come testo (per il replay delle registrazioni).
#[tauri::command]
fn leggi_file(percorso: String) -> Result<String, String> {
    std::fs::read_to_string(&percorso).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Chiavi SSH
// ---------------------------------------------------------------------------

#[tauri::command]
fn genera_chiave(nome: String, commento: String) -> Result<String, String> {
    oxiterm_core::chiavi::genera(&nome, &commento)
}

#[tauri::command]
fn lista_chiavi() -> Vec<oxiterm_core::chiavi::ChiavePub> {
    oxiterm_core::chiavi::lista()
}

/// Copia una chiave pubblica nel ~/.ssh/authorized_keys del server (ssh-copy-id).
#[tauri::command]
async fn copia_chiave(
    stato: State<'_, Sessioni>,
    id: String,
    pubblica: String,
) -> Result<String, String> {
    let mappa = stato.0.lock().await;
    let s = mappa.get(&id).ok_or("sessione inesistente")?;
    let conn = s.ssh.as_ref().ok_or("serve una sessione SSH aperta")?;
    let pulita = pubblica.replace('\'', "");
    let cmd = format!(
        "mkdir -p ~/.ssh && chmod 700 ~/.ssh && echo '{pulita}' >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys && echo OK"
    );
    conn.esegui(&cmd).await
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

/// Carica un file mostrando l'avanzamento (eventi `sftp-progresso`).
#[tauri::command]
async fn sftp_carica_coda(
    app: tauri::AppHandle,
    stato: State<'_, Sessioni>,
    id: String,
    trasferimento: String,
    locale: String,
    remoto: String,
) -> Result<(), String> {
    con_sftp(&stato, &id, move |c| async move {
        let r = sftp::carica_progresso(&c, &locale, &remoto, |fatti, totale| {
            let _ = app.emit(
                "sftp-progresso",
                ProgressoSftp {
                    id: trasferimento.clone(),
                    fatti,
                    totale,
                },
            );
        })
        .await;
        (c, r)
    })
    .await
}

/// Scarica un file mostrando l'avanzamento (eventi `sftp-progresso`).
#[tauri::command]
async fn sftp_scarica_coda(
    app: tauri::AppHandle,
    stato: State<'_, Sessioni>,
    id: String,
    trasferimento: String,
    remoto: String,
    locale: String,
) -> Result<(), String> {
    con_sftp(&stato, &id, move |c| async move {
        let r = sftp::scarica_progresso(&c, &remoto, &locale, |fatti, totale| {
            let _ = app.emit(
                "sftp-progresso",
                ProgressoSftp {
                    id: trasferimento.clone(),
                    fatti,
                    totale,
                },
            );
        })
        .await;
        (c, r)
    })
    .await
}

/// Legge un file remoto come testo (editor integrato).
#[tauri::command]
async fn sftp_leggi_testo(
    stato: State<'_, Sessioni>,
    id: String,
    remoto: String,
) -> Result<String, String> {
    con_sftp(&stato, &id, move |c| async move {
        let r = sftp::leggi_testo(&c, &remoto).await;
        (c, r)
    })
    .await
}

/// Salva del testo in un file remoto (editor integrato).
#[tauri::command]
async fn sftp_scrivi_testo(
    stato: State<'_, Sessioni>,
    id: String,
    remoto: String,
    contenuto: String,
) -> Result<(), String> {
    con_sftp(&stato, &id, move |c| async move {
        let r = sftp::scrivi_testo(&c, &remoto, &contenuto).await;
        (c, r)
    })
    .await
}

/// Legge un file remoto e lo restituisce in base64 (anteprima immagini).
#[tauri::command]
async fn sftp_leggi_base64(
    stato: State<'_, Sessioni>,
    id: String,
    remoto: String,
) -> Result<String, String> {
    con_sftp(&stato, &id, move |c| async move {
        let r = sftp::leggi_bytes(&c, &remoto)
            .await
            .map(|b| base64::engine::general_purpose::STANDARD.encode(b));
        (c, r)
    })
    .await
}

/// Carica ricorsivamente una cartella locale.
#[tauri::command]
async fn sftp_carica_cartella(
    stato: State<'_, Sessioni>,
    id: String,
    locale: String,
    remoto: String,
) -> Result<(), String> {
    con_sftp(&stato, &id, move |c| async move {
        let r = sftp::carica_cartella(&c, &locale, &remoto).await;
        (c, r)
    })
    .await
}

/// Scarica ricorsivamente una cartella remota.
#[tauri::command]
async fn sftp_scarica_cartella(
    stato: State<'_, Sessioni>,
    id: String,
    remoto: String,
    locale: String,
) -> Result<(), String> {
    con_sftp(&stato, &id, move |c| async move {
        let r = sftp::scarica_cartella(&c, &remoto, &locale).await;
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

/// Importa gli host da ~/.ssh/config nella rubrica. Ritorna quanti aggiunti.
#[tauri::command]
fn importa_ssh_config(app: tauri::AppHandle) -> Result<usize, String> {
    let p = oxiterm_core::chiavi::dir_ssh().join("config");
    let testo = std::fs::read_to_string(&p)
        .map_err(|e| format!("impossibile leggere ~/.ssh/config: {e}"))?;
    let nuove = oxiterm_core::sshconfig::parse(&testo);
    let file = file_sessioni(&app)?;
    let mut tutte = storage::carica_sessioni(&file);
    let mut aggiunte = 0;
    for n in nuove {
        match tutte.iter_mut().find(|s| s.id == n.id) {
            Some(e) => *e = n,
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
// Segnalibri SFTP (sincronizzabili: salvati nella cartella di config)
// ---------------------------------------------------------------------------

fn file_segnalibri(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("segnalibri.json"))
}

#[tauri::command]
fn lista_segnalibri(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    Ok(std::fs::read_to_string(file_segnalibri(&app)?)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default())
}

#[tauri::command]
fn salva_segnalibri(app: tauri::AppHandle, lista: Vec<String>) -> Result<(), String> {
    let f = file_segnalibri(&app)?;
    if let Some(d) = f.parent() {
        std::fs::create_dir_all(d).ok();
    }
    std::fs::write(f, serde_json::to_string_pretty(&lista).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
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
// Vault cifrato delle password
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct StatoVaultView {
    esiste: bool,
    sbloccato: bool,
}

#[tauri::command]
fn vault_stato(app: tauri::AppHandle, stato: State<'_, StatoVault>) -> Result<StatoVaultView, String> {
    Ok(StatoVaultView {
        esiste: vault::esiste(&file_vault(&app)?),
        sbloccato: stato.0.lock().unwrap().is_some(),
    })
}

#[tauri::command]
fn vault_sblocca(
    app: tauri::AppHandle,
    stato: State<'_, StatoVault>,
    master: String,
) -> Result<(), String> {
    let chiave = vault::sblocca(&file_vault(&app)?, &master)?;
    *stato.0.lock().unwrap() = Some(chiave);
    Ok(())
}

#[tauri::command]
fn vault_blocca(stato: State<'_, StatoVault>) {
    *stato.0.lock().unwrap() = None;
}

#[tauri::command]
fn vault_salva_password(
    app: tauri::AppHandle,
    stato: State<'_, StatoVault>,
    id: String,
    password: String,
) -> Result<(), String> {
    let chiave = (*stato.0.lock().unwrap()).ok_or("vault bloccato")?;
    vault::salva_password(&file_vault(&app)?, &chiave, &id, &password)
}

#[tauri::command]
fn vault_leggi_password(
    app: tauri::AppHandle,
    stato: State<'_, StatoVault>,
    id: String,
) -> Result<Option<String>, String> {
    let chiave = (*stato.0.lock().unwrap()).ok_or("vault bloccato")?;
    vault::leggi_password(&file_vault(&app)?, &chiave, &id)
}

// ---------------------------------------------------------------------------
// Sincronizzazione cloud (Git)
// ---------------------------------------------------------------------------

/// Cartella di configurazione (il repository di sincronizzazione).
fn cartella_config(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

#[tauri::command]
fn sync_remote(app: tauri::AppHandle) -> Result<Option<String>, String> {
    Ok(git_sync::remote_url(&cartella_config(&app)?))
}

#[tauri::command]
fn sync_imposta_remote(app: tauri::AppHandle, url: String) -> Result<(), String> {
    git_sync::imposta_remote(&cartella_config(&app)?, &url)
}

#[tauri::command]
fn sync_push(app: tauri::AppHandle) -> Result<(), String> {
    git_sync::push(&cartella_config(&app)?)
}

#[tauri::command]
fn sync_pull(app: tauri::AppHandle) -> Result<(), String> {
    git_sync::pull(&cartella_config(&app)?)
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
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(Sessioni::default())
        .manage(StatoVnc::default())
        .manage(StatoVault::default())
        .invoke_handler(tauri::generate_handler![
            ssh_connetti,
            apri_locale,
            apri_telnet,
            apri_seriale,
            porte_seriali,
            apri_vnc,
            vnc_mouse,
            vnc_tasto,
            vnc_chiudi,
            net_porta,
            net_ping,
            net_traceroute,
            net_wol,
            server_stato,
            term_scrivi,
            term_ridimensiona,
            term_chiudi,
            term_log_avvia,
            term_log_ferma,
            term_rec_avvia,
            term_rec_ferma,
            leggi_file,
            genera_chiave,
            lista_chiavi,
            copia_chiave,
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
            sftp_carica_coda,
            sftp_scarica_coda,
            sftp_apri_editor,
            sftp_leggi_testo,
            sftp_scrivi_testo,
            sftp_carica_cartella,
            sftp_scarica_cartella,
            sftp_leggi_base64,
            lista_sessioni,
            salva_sessione,
            elimina_sessione,
            importa_ssh_config,
            lista_segnalibri,
            salva_segnalibri,
            esporta_rubrica,
            importa_rubrica,
            lista_snippet,
            salva_snippet,
            elimina_snippet,
            sync_remote,
            sync_imposta_remote,
            sync_push,
            sync_pull,
            vault_stato,
            vault_sblocca,
            vault_blocca,
            vault_salva_password,
            vault_leggi_password,
        ])
        .run(tauri::generate_context!())
        .expect("errore durante l'avvio di Oxiterm");
}
