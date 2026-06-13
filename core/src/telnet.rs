//! Client Telnet minimale.
//!
//! Telnet usa byte di controllo (IAC = 0xFF) per negoziare opzioni. Qui facciamo
//! una negoziazione semplice: rifiutiamo tutte le opzioni che il server propone
//! (rispondiamo WONT/DONT). È sufficiente per collegarsi a moltissimi servizi
//! e apparati di rete in modalità riga.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::term::{Canale, ComandoTerm};

const IAC: u8 = 255;
const DONT: u8 = 254;
const DO: u8 = 253;
const WONT: u8 = 252;
const WILL: u8 = 251;
const SB: u8 = 250;
const SE: u8 = 240;

/// Apre una connessione Telnet verso host:porta.
pub async fn apri(host: &str, porta: u16) -> Result<Canale, String> {
    let stream = TcpStream::connect((host, porta))
        .await
        .map_err(|e| format!("connessione Telnet fallita: {e}"))?;
    let (mut lettore, mut scrittore) = stream.into_split();

    let (tx_in, mut rx_in) = mpsc::channel::<ComandoTerm>(64);
    let (tx_out, rx_out) = mpsc::channel::<Vec<u8>>(256);

    tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            tokio::select! {
                letto = lettore.read(&mut buf) => match letto {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let (puliti, risposte) = filtra_iac(&buf[..n]);
                        if !risposte.is_empty() { let _ = scrittore.write_all(&risposte).await; }
                        if !puliti.is_empty() && tx_out.send(puliti).await.is_err() { break; }
                    }
                },
                cmd = rx_in.recv() => match cmd {
                    Some(ComandoTerm::Scrivi(b)) => {
                        // Raddoppia gli eventuali 0xFF nei dati utente (escape IAC).
                        let mut fuori = Vec::with_capacity(b.len());
                        for byte in b { fuori.push(byte); if byte == IAC { fuori.push(IAC); } }
                        if scrittore.write_all(&fuori).await.is_err() { break; }
                    }
                    Some(ComandoTerm::Ridimensiona(_, _)) => {} // Telnet base: niente resize
                    Some(ComandoTerm::Chiudi) | None => break,
                },
            }
        }
    });

    Ok(Canale {
        input: tx_in,
        output: rx_out,
    })
}

/// Estrae i dati "veri" da un blocco Telnet e prepara le risposte di rifiuto
/// per ogni opzione negoziata dal server.
fn filtra_iac(dati: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut puliti = Vec::with_capacity(dati.len());
    let mut risposte = Vec::new();
    let mut i = 0;
    while i < dati.len() {
        if dati[i] != IAC {
            puliti.push(dati[i]);
            i += 1;
            continue;
        }
        // Sequenza IAC.
        if i + 1 >= dati.len() {
            break;
        }
        match dati[i + 1] {
            IAC => {
                puliti.push(IAC); // IAC IAC = un singolo 0xFF nei dati
                i += 2;
            }
            DO | DONT => {
                // Il server ci chiede di abilitare/disabilitare: rispondiamo WONT.
                if i + 2 < dati.len() {
                    risposte.extend_from_slice(&[IAC, WONT, dati[i + 2]]);
                    i += 3;
                } else {
                    break;
                }
            }
            WILL | WONT => {
                // Il server dice cosa farà lui: rispondiamo DONT.
                if i + 2 < dati.len() {
                    risposte.extend_from_slice(&[IAC, DONT, dati[i + 2]]);
                    i += 3;
                } else {
                    break;
                }
            }
            SB => {
                // Sotto-negoziazione: salta fino a IAC SE.
                i += 2;
                while i + 1 < dati.len() && !(dati[i] == IAC && dati[i + 1] == SE) {
                    i += 1;
                }
                i += 2;
            }
            _ => {
                i += 2;
            }
        }
    }
    (puliti, risposte)
}
