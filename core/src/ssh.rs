//! Connessione SSH: shell con PTY, SFTP e tunnel (port forwarding).
//!
//! Una `Connessione` tiene aperta la sessione TCP autenticata; da essa si
//! aprono più canali: una shell (terminale), un SFTP, o dei tunnel.
//!
//! Sicurezza: la chiave del server viene verificata in stile TOFU
//! (Trust On First Use) contro un file `known_hosts` JSON: la prima volta si
//! memorizza, le volte dopo si rifiuta la connessione se la chiave è cambiata.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use russh::client::{self, Handle};
use russh::keys::PrivateKeyWithHashAlg;
use russh::ChannelMsg;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use crate::model::Auth;
use crate::term::{Canale, ComandoTerm};

// Ri-esporta il tipo SFTP così il livello desktop non dipende da russh_sftp.
pub use russh_sftp::client::SftpSession;

/// Gestore degli eventi del client SSH, con verifica known_hosts.
struct Gestore {
    /// File JSON host -> chiave pubblica (formato OpenSSH).
    conosciuti: PathBuf,
    /// Etichetta "host:porta" della connessione in corso.
    etichetta: String,
}

impl client::Handler for Gestore {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        chiave: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        let attuale = chiave.to_openssh().unwrap_or_default();
        let mut mappa = carica_conosciuti(&self.conosciuti);
        match mappa.get(&self.etichetta) {
            // Già vista: deve combaciare, altrimenti rifiuto (possibile attacco).
            Some(salvata) => Ok(salvata == &attuale),
            // Prima volta: la memorizzo e accetto.
            None => {
                mappa.insert(self.etichetta.clone(), attuale);
                salva_conosciuti(&self.conosciuti, &mappa);
                Ok(true)
            }
        }
    }
}

fn carica_conosciuti(file: &PathBuf) -> HashMap<String, String> {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn salva_conosciuti(file: &PathBuf, mappa: &HashMap<String, String>) {
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(testo) = serde_json::to_string_pretty(mappa) {
        let _ = std::fs::write(file, testo);
    }
}

/// Una connessione SSH autenticata e pronta ad aprire canali.
/// Il `Handle` è in un `Arc` perché i tunnel lo condividono fra più task.
pub struct Connessione {
    handle: Arc<Handle<Gestore>>,
}

impl Connessione {
    /// Apre la connessione TCP, esegue l'handshake (con verifica known_hosts)
    /// e l'autenticazione.
    pub async fn connetti(
        host: &str,
        porta: u16,
        utente: &str,
        auth: Auth,
        known_hosts: PathBuf,
    ) -> Result<Self, String> {
        // Keepalive: un "ping" ogni 15s evita che il firewall chiuda la sessione.
        let config = Arc::new(client::Config {
            keepalive_interval: Some(std::time::Duration::from_secs(15)),
            ..Default::default()
        });
        let gestore = Gestore {
            conosciuti: known_hosts,
            etichetta: format!("{host}:{porta}"),
        };
        let mut handle = client::connect(config, (host, porta), gestore)
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
            return Err("autenticazione fallita (credenziali errate?)".into());
        }
        Ok(Connessione {
            handle: Arc::new(handle),
        })
    }

    /// Apre una shell interattiva con PTY e avvia il task ponte.
    pub async fn apri_shell(&self, colonne: u32, righe: u32) -> Result<Canale, String> {
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

        let (tx_in, mut rx_in) = mpsc::channel::<ComandoTerm>(64);
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
                        Some(ComandoTerm::Scrivi(b)) => { let _ = canale.data(&b[..]).await; }
                        Some(ComandoTerm::Ridimensiona(c, r)) => {
                            let _ = canale.window_change(c, r, 0, 0).await;
                        }
                        Some(ComandoTerm::Chiudi) | None => {
                            let _ = canale.eof().await;
                            break;
                        }
                    },
                }
            }
        });

        Ok(Canale {
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

    /// Tunnel locale (-L): tutto ciò che arriva su `porta_locale` viene inoltrato,
    /// attraverso l'SSH, verso `host_remoto:porta_remota`.
    pub async fn tunnel_locale(
        &self,
        porta_locale: u16,
        host_remoto: String,
        porta_remota: u16,
    ) -> Result<StopTunnel, String> {
        let listener = TcpListener::bind(("127.0.0.1", porta_locale))
            .await
            .map_err(|e| format!("impossibile aprire la porta {porta_locale}: {e}"))?;
        let handle = self.handle.clone();
        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop_rx.recv() => break,
                    acc = listener.accept() => {
                        let Ok((mut sock, addr)) = acc else { continue };
                        let h = handle.clone();
                        let hr = host_remoto.clone();
                        tokio::spawn(async move {
                            if let Ok(canale) = h
                                .channel_open_direct_tcpip(hr, porta_remota as u32, addr.ip().to_string(), addr.port() as u32)
                                .await
                            {
                                let mut stream = canale.into_stream();
                                let _ = tokio::io::copy_bidirectional(&mut sock, &mut stream).await;
                            }
                        });
                    }
                }
            }
        });
        Ok(StopTunnel(stop_tx))
    }

    /// Proxy SOCKS5 dinamico (-D): apre un proxy SOCKS5 locale che instrada
    /// ogni connessione attraverso l'SSH (utile come "VPN al volo").
    pub async fn tunnel_socks(&self, porta_locale: u16) -> Result<StopTunnel, String> {
        let listener = TcpListener::bind(("127.0.0.1", porta_locale))
            .await
            .map_err(|e| format!("impossibile aprire la porta {porta_locale}: {e}"))?;
        let handle = self.handle.clone();
        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop_rx.recv() => break,
                    acc = listener.accept() => {
                        let Ok((sock, addr)) = acc else { continue };
                        let h = handle.clone();
                        tokio::spawn(async move {
                            let _ = gestisci_socks(sock, addr, h).await;
                        });
                    }
                }
            }
        });
        Ok(StopTunnel(stop_tx))
    }
}

/// Quando viene rilasciato (o si chiama `ferma`), il tunnel smette di accettare.
pub struct StopTunnel(mpsc::Sender<()>);

impl StopTunnel {
    pub async fn ferma(&self) {
        let _ = self.0.send(()).await;
    }
}

/// Gestisce una singola connessione SOCKS5 e la inoltra via SSH.
async fn gestisci_socks(
    mut sock: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
    handle: Arc<Handle<Gestore>>,
) -> Result<(), String> {
    // 1) Saluto: versione + metodi di autenticazione. Rispondiamo "nessuna auth".
    let mut testa = [0u8; 2];
    sock.read_exact(&mut testa).await.map_err(|e| e.to_string())?;
    if testa[0] != 0x05 {
        return Err("non è SOCKS5".into());
    }
    let nmetodi = testa[1] as usize;
    let mut metodi = vec![0u8; nmetodi];
    sock.read_exact(&mut metodi).await.map_err(|e| e.to_string())?;
    sock.write_all(&[0x05, 0x00]).await.map_err(|e| e.to_string())?;

    // 2) Richiesta: ver, cmd, rsv, atyp, indirizzo, porta.
    let mut req = [0u8; 4];
    sock.read_exact(&mut req).await.map_err(|e| e.to_string())?;
    if req[1] != 0x01 {
        // solo CONNECT è supportato
        sock.write_all(&[0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await.ok();
        return Err("comando SOCKS non supportato".into());
    }
    let host = match req[3] {
        0x01 => {
            let mut a = [0u8; 4];
            sock.read_exact(&mut a).await.map_err(|e| e.to_string())?;
            std::net::Ipv4Addr::from(a).to_string()
        }
        0x03 => {
            let mut len = [0u8; 1];
            sock.read_exact(&mut len).await.map_err(|e| e.to_string())?;
            let mut nome = vec![0u8; len[0] as usize];
            sock.read_exact(&mut nome).await.map_err(|e| e.to_string())?;
            String::from_utf8_lossy(&nome).to_string()
        }
        0x04 => {
            let mut a = [0u8; 16];
            sock.read_exact(&mut a).await.map_err(|e| e.to_string())?;
            std::net::Ipv6Addr::from(a).to_string()
        }
        _ => return Err("tipo di indirizzo SOCKS sconosciuto".into()),
    };
    let mut porta_buf = [0u8; 2];
    sock.read_exact(&mut porta_buf).await.map_err(|e| e.to_string())?;
    let porta = u16::from_be_bytes(porta_buf);

    // 3) Apri il canale SSH verso la destinazione e rispondi "ok".
    let canale = handle
        .channel_open_direct_tcpip(host, porta as u32, addr.ip().to_string(), addr.port() as u32)
        .await
        .map_err(|e| e.to_string())?;
    sock.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await
        .map_err(|e| e.to_string())?;

    // 4) Copia bidirezionale.
    let mut stream = canale.into_stream();
    let _ = tokio::io::copy_bidirectional(&mut sock, &mut stream).await;
    Ok(())
}
