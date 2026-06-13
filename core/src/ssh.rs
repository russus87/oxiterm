//! Connessione SSH e shell interattiva.
//!
//! Una `Connessione` tiene aperta la sessione TCP autenticata; da essa si
//! possono aprire più canali: una shell con PTY (terminale) e/o un SFTP.
//!
//! La shell gira in un task dedicato che fa da ponte: riceve i comandi del
//! frontend (scrivi/ridimensiona/chiudi) su un canale e rimanda l'output del
//! server su un altro canale. Così il `core` resta indipendente da Tauri:
//! sarà il livello desktop a inoltrare l'output come eventi alla UI.

use std::sync::Arc;

use russh::client::{self, Handle};
use russh::keys::PrivateKeyWithHashAlg;
use russh::ChannelMsg;
use tokio::sync::mpsc;

use crate::model::Auth;

// Ri-esporta il tipo SFTP così il livello desktop non deve dipendere
// direttamente dal crate russh_sftp.
pub use russh_sftp::client::SftpSession;

/// Comando inviato alla shell in esecuzione.
pub enum ComandoSsh {
    /// Byte digitati dall'utente da scrivere sul canale.
    Scrivi(Vec<u8>),
    /// Nuova dimensione del terminale (colonne, righe).
    Ridimensiona(u32, u32),
    /// Chiudi la shell.
    Chiudi,
}

/// Estremità di una shell aperta: si scrive su `input`, si legge da `output`.
pub struct CanaleShell {
    pub input: mpsc::Sender<ComandoSsh>,
    pub output: mpsc::Receiver<Vec<u8>>,
}

/// Gestore degli eventi del client SSH. Per ora accetta qualsiasi chiave del
/// server (nessuna verifica known_hosts: da migliorare in una fase futura).
struct Gestore;

impl client::Handler for Gestore {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _chiave: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

/// Una connessione SSH autenticata e pronta ad aprire canali.
pub struct Connessione {
    handle: Handle<Gestore>,
}

impl Connessione {
    /// Apre la connessione TCP, esegue l'handshake e l'autenticazione.
    pub async fn connetti(
        host: &str,
        porta: u16,
        utente: &str,
        auth: Auth,
    ) -> Result<Self, String> {
        let config = Arc::new(client::Config::default());
        let mut handle = client::connect(config, (host, porta), Gestore)
            .await
            .map_err(|e| format!("connessione fallita: {e}"))?;

        let esito = match auth {
            Auth::Password { password } => handle
                .authenticate_password(utente, password)
                .await
                .map_err(|e| e.to_string())?,
            Auth::Chiave {
                percorso,
                passphrase,
            } => {
                let chiave = russh::keys::load_secret_key(&percorso, passphrase.as_deref())
                    .map_err(|e| format!("chiave non valida: {e}"))?;
                handle
                    .authenticate_publickey(
                        utente,
                        PrivateKeyWithHashAlg::new(Arc::new(chiave), None),
                    )
                    .await
                    .map_err(|e| e.to_string())?
            }
        };

        if !esito.success() {
            return Err("autenticazione fallita".into());
        }
        Ok(Connessione { handle })
    }

    /// Apre una shell interattiva con PTY e avvia il task ponte.
    pub async fn apri_shell(&self, colonne: u32, righe: u32) -> Result<CanaleShell, String> {
        let mut canale = self
            .handle
            .channel_open_session()
            .await
            .map_err(|e| e.to_string())?;
        canale
            .request_pty(false, "xterm-256color", colonne, righe, 0, 0, &[])
            .await
            .map_err(|e| e.to_string())?;
        canale
            .request_shell(false)
            .await
            .map_err(|e| e.to_string())?;

        let (tx_in, mut rx_in) = mpsc::channel::<ComandoSsh>(64);
        let (tx_out, rx_out) = mpsc::channel::<Vec<u8>>(256);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = canale.wait() => match msg {
                        Some(ChannelMsg::Data { data }) => {
                            if tx_out.send(data.to_vec()).await.is_err() { break; }
                        }
                        Some(ChannelMsg::ExtendedData { data, .. }) => {
                            if tx_out.send(data.to_vec()).await.is_err() { break; }
                        }
                        Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => break,
                        _ => {}
                    },
                    cmd = rx_in.recv() => match cmd {
                        Some(ComandoSsh::Scrivi(b)) => { let _ = canale.data(&b[..]).await; }
                        Some(ComandoSsh::Ridimensiona(c, r)) => {
                            let _ = canale.window_change(c, r, 0, 0).await;
                        }
                        Some(ComandoSsh::Chiudi) | None => {
                            let _ = canale.eof().await;
                            break;
                        }
                    },
                }
            }
        });

        Ok(CanaleShell {
            input: tx_in,
            output: rx_out,
        })
    }

    /// Apre un canale SFTP sulla stessa connessione.
    pub async fn apri_sftp(&self) -> Result<SftpSession, String> {
        let canale = self
            .handle
            .channel_open_session()
            .await
            .map_err(|e| e.to_string())?;
        canale
            .request_subsystem(true, "sftp")
            .await
            .map_err(|e| e.to_string())?;
        SftpSession::new(canale.into_stream())
            .await
            .map_err(|e| e.to_string())
    }
}
