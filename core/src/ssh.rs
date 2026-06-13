//! Connessione SSH: shell con PTY, SFTP e tunnel (port forwarding L/R/D).
//!
//! Sicurezza: la chiave del server viene verificata contro un file `known_hosts`
//! JSON. Se è sconosciuta o cambiata, la connessione viene rifiutata e si segnala
//! l'esito al chiamante (errore `HOSTKEY:<stato>:<impronta>`), così la UI può
//! chiedere conferma all'utente e ritentare con un "modo di fiducia" diverso.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use russh::client::{self, Handle};
use russh::keys::PrivateKeyWithHashAlg;
use russh::{Channel, ChannelMsg};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use crate::model::{Auth, JumpConfig};
use crate::term::{Canale, ComandoTerm};

// Ri-esporta il tipo SFTP così il livello desktop non dipende da russh_sftp.
pub use russh_sftp::client::SftpSession;

/// Quanto fidarsi della chiave del server in fase di connessione.
#[derive(Clone, Copy)]
pub enum ModoFiducia {
    /// Comportamento normale: rifiuta chiavi sconosciute o cambiate.
    Normale,
    /// Accetta e memorizza una chiave sconosciuta (prima connessione).
    AccettaNuova,
    /// Sostituisci una chiave cambiata (l'utente sa cosa sta facendo).
    Sostituisci,
}

/// Mappa porta-remota -> (host, porta) locali, per i forward remoti (-R).
type Inoltri = Arc<Mutex<HashMap<u16, (String, u16)>>>;

/// Gestore degli eventi del client SSH.
struct Gestore {
    conosciuti: PathBuf,
    etichetta: String,
    modo: ModoFiducia,
    /// Esito della verifica chiave, letto dal chiamante se la connessione fallisce.
    esito: Arc<Mutex<Option<(String, String)>>>,
    inoltri: Inoltri,
}

impl client::Handler for Gestore {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        chiave: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        let attuale = chiave.to_openssh().unwrap_or_default();
        let impronta = chiave
            .fingerprint(russh::keys::ssh_key::HashAlg::Sha256)
            .to_string();
        let mut mappa = carica_conosciuti(&self.conosciuti);

        match mappa.get(&self.etichetta) {
            Some(salvata) if salvata == &attuale => Ok(true),
            Some(_) => match self.modo {
                ModoFiducia::Sostituisci => {
                    mappa.insert(self.etichetta.clone(), attuale);
                    salva_conosciuti(&self.conosciuti, &mappa);
                    Ok(true)
                }
                _ => {
                    *self.esito.lock().unwrap() = Some(("cambiata".into(), impronta));
                    Ok(false)
                }
            },
            None => match self.modo {
                ModoFiducia::AccettaNuova | ModoFiducia::Sostituisci => {
                    mappa.insert(self.etichetta.clone(), attuale);
                    salva_conosciuti(&self.conosciuti, &mappa);
                    Ok(true)
                }
                ModoFiducia::Normale => {
                    *self.esito.lock().unwrap() = Some(("nuova".into(), impronta));
                    Ok(false)
                }
            },
        }
    }

    /// Connessione in arrivo da un forward remoto (-R): la colleghiamo al
    /// bersaglio locale configurato per quella porta.
    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: Channel<client::Msg>,
        _connected_address: &str,
        connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        let bersaglio = self
            .inoltri
            .lock()
            .unwrap()
            .get(&(connected_port as u16))
            .cloned();
        if let Some((host, porta)) = bersaglio {
            tokio::spawn(async move {
                if let Ok(mut tcp) = tokio::net::TcpStream::connect((host, porta)).await {
                    let mut stream = channel.into_stream();
                    let _ = tokio::io::copy_bidirectional(&mut tcp, &mut stream).await;
                }
            });
        }
        Ok(())
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
/// Il `Handle` è in un Mutex async perché alcune richieste (tcpip_forward)
/// vogliono `&mut`, ed è condiviso fra più task (tunnel).
pub struct Connessione {
    handle: Arc<tokio::sync::Mutex<Handle<Gestore>>>,
    inoltri: Inoltri,
    /// Eventuale connessione al bastion, tenuta viva finché vive questa.
    _bastione: Option<Box<Connessione>>,
}

impl Connessione {
    /// Apre la connessione SSH (direttamente o attraverso un jump host),
    /// verifica la chiave del server e autentica.
    pub async fn connetti(
        host: &str,
        porta: u16,
        utente: &str,
        auth: Auth,
        known_hosts: PathBuf,
        modo: ModoFiducia,
        jump: Option<JumpConfig>,
    ) -> Result<Self, String> {
        let config = Arc::new(client::Config {
            keepalive_interval: Some(std::time::Duration::from_secs(15)),
            ..Default::default()
        });
        let esito = Arc::new(Mutex::new(None));
        let inoltri: Inoltri = Arc::new(Mutex::new(HashMap::new()));
        let gestore = Gestore {
            conosciuti: known_hosts.clone(),
            etichetta: format!("{host}:{porta}"),
            modo,
            esito: esito.clone(),
            inoltri: inoltri.clone(),
        };

        // Converte un errore di connessione tenendo conto della chiave del server.
        let err_connessione = |e: russh::Error, esito: &Arc<Mutex<Option<(String, String)>>>| {
            if let Some((stato, impronta)) = esito.lock().unwrap().clone() {
                format!("HOSTKEY:{stato}:{impronta}")
            } else {
                format!("connessione fallita: {e}")
            }
        };

        // Con jump host: ci colleghiamo prima al bastion, apriamo un canale
        // verso il bersaglio e ci facciamo l'handshake SSH sopra quel canale.
        let mut bastione_vivo: Option<Box<Connessione>> = None;
        let mut handle = if let Some(j) = jump {
            let bastione = Box::pin(Connessione::connetti(
                &j.host,
                j.porta,
                &j.utente,
                j.auth,
                known_hosts,
                modo,
                None,
            ))
            .await?;
            let canale = bastione
                .handle
                .lock()
                .await
                .channel_open_direct_tcpip(host.to_string(), porta as u32, "127.0.0.1".to_string(), 0)
                .await
                .map_err(|e| format!("il jump host non raggiunge {host}:{porta}: {e}"))?;
            let stream = canale.into_stream();
            let h = client::connect_stream(config, stream, gestore)
                .await
                .map_err(|e| err_connessione(e, &esito))?;
            bastione_vivo = Some(Box::new(bastione));
            h
        } else {
            client::connect(config, (host, porta), gestore)
                .await
                .map_err(|e| err_connessione(e, &esito))?
        };

        let ok = match auth {
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

        if !ok.success() {
            return Err("autenticazione fallita (credenziali errate?)".into());
        }
        Ok(Connessione {
            handle: Arc::new(tokio::sync::Mutex::new(handle)),
            inoltri,
            _bastione: bastione_vivo,
        })
    }

    /// Apre una shell interattiva con PTY e avvia il task ponte.
    pub async fn apri_shell(&self, colonne: u32, righe: u32) -> Result<Canale, String> {
        let mut canale = self
            .handle
            .lock()
            .await
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
            .lock()
            .await
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

    /// Tunnel locale (-L): porta locale -> host:porta remoti, via SSH.
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
                            let aperto = {
                                h.lock().await
                                    .channel_open_direct_tcpip(hr, porta_remota as u32, addr.ip().to_string(), addr.port() as u32)
                                    .await
                            };
                            if let Ok(canale) = aperto {
                                let mut stream = canale.into_stream();
                                let _ = tokio::io::copy_bidirectional(&mut sock, &mut stream).await;
                            }
                        });
                    }
                }
            }
        });
        Ok(StopTunnel(StopInterno::Locale(stop_tx)))
    }

    /// Proxy SOCKS5 dinamico (-D): proxy locale che instrada via SSH.
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
        Ok(StopTunnel(StopInterno::Locale(stop_tx)))
    }

    /// Forward remoto (-R): il server ascolta su `porta_remota` e inoltra a
    /// `host_locale:porta_locale` (visto da noi).
    pub async fn tunnel_remoto(
        &self,
        porta_remota: u16,
        host_locale: String,
        porta_locale: u16,
    ) -> Result<StopTunnel, String> {
        self.inoltri
            .lock()
            .unwrap()
            .insert(porta_remota, (host_locale, porta_locale));
        self.handle
            .lock()
            .await
            .tcpip_forward("0.0.0.0", porta_remota as u32)
            .await
            .map_err(|e| format!("il server ha rifiutato il forward remoto: {e}"))?;
        Ok(StopTunnel(StopInterno::Remoto {
            handle: self.handle.clone(),
            porta: porta_remota,
            inoltri: self.inoltri.clone(),
        }))
    }
}

/// Handle per fermare un tunnel. Il dettaglio interno è privato per non
/// esporre il tipo `Gestore`.
pub struct StopTunnel(StopInterno);

enum StopInterno {
    Locale(mpsc::Sender<()>),
    Remoto {
        handle: Arc<tokio::sync::Mutex<Handle<Gestore>>>,
        porta: u16,
        inoltri: Inoltri,
    },
}

impl StopTunnel {
    pub async fn ferma(&self) {
        match &self.0 {
            StopInterno::Locale(tx) => {
                let _ = tx.send(()).await;
            }
            StopInterno::Remoto {
                handle,
                porta,
                inoltri,
            } => {
                let _ = handle.lock().await.cancel_tcpip_forward("0.0.0.0", *porta as u32).await;
                inoltri.lock().unwrap().remove(porta);
            }
        }
    }
}

/// Gestisce una singola connessione SOCKS5 e la inoltra via SSH.
async fn gestisci_socks(
    mut sock: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
    handle: Arc<tokio::sync::Mutex<Handle<Gestore>>>,
) -> Result<(), String> {
    let mut testa = [0u8; 2];
    sock.read_exact(&mut testa).await.map_err(|e| e.to_string())?;
    if testa[0] != 0x05 {
        return Err("non è SOCKS5".into());
    }
    let nmetodi = testa[1] as usize;
    let mut metodi = vec![0u8; nmetodi];
    sock.read_exact(&mut metodi).await.map_err(|e| e.to_string())?;
    sock.write_all(&[0x05, 0x00]).await.map_err(|e| e.to_string())?;

    let mut req = [0u8; 4];
    sock.read_exact(&mut req).await.map_err(|e| e.to_string())?;
    if req[1] != 0x01 {
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

    let canale = {
        handle
            .lock()
            .await
            .channel_open_direct_tcpip(host, porta as u32, addr.ip().to_string(), addr.port() as u32)
            .await
            .map_err(|e| e.to_string())?
    };
    sock.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await
        .map_err(|e| e.to_string())?;

    let mut stream = canale.into_stream();
    let _ = tokio::io::copy_bidirectional(&mut sock, &mut stream).await;
    Ok(())
}
