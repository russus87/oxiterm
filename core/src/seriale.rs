//! Console seriale: apre una porta COM / tty a un certo baud rate.
//! Usa il crate `serialport` (multipiattaforma).

use std::io::{Read, Write};
use std::time::Duration;

use tokio::sync::mpsc;

use crate::term::{Canale, ComandoTerm};

/// Elenca le porte seriali disponibili sul sistema.
pub fn porte() -> Vec<String> {
    serialport::available_ports()
        .map(|v| v.into_iter().map(|p| p.port_name).collect())
        .unwrap_or_default()
}

/// Apre una porta seriale e la espone come un terminale.
pub fn apri(porta: &str, baud: u32) -> Result<Canale, String> {
    let porta_aperta = serialport::new(porta, baud)
        .timeout(Duration::from_millis(50))
        .open()
        .map_err(|e| format!("impossibile aprire {porta}: {e}"))?;

    // Un clone per leggere, l'originale per scrivere.
    let mut lettore = porta_aperta
        .try_clone()
        .map_err(|e| e.to_string())?;
    let mut scrittore = porta_aperta;

    let (tx_in, mut rx_in) = mpsc::channel::<ComandoTerm>(64);
    let (tx_out, rx_out) = mpsc::channel::<Vec<u8>>(256);

    // Lettura bloccante in un thread dedicato (il timeout evita di bloccarsi per sempre).
    std::thread::spawn(move || {
        let mut buf = [0u8; 2048];
        loop {
            match lettore.read(&mut buf) {
                Ok(0) => {}
                Ok(n) => {
                    if tx_out.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(_) => break,
            }
        }
    });

    tokio::spawn(async move {
        while let Some(cmd) = rx_in.recv().await {
            match cmd {
                ComandoTerm::Scrivi(b) => {
                    let _ = scrittore.write_all(&b);
                    let _ = scrittore.flush();
                }
                ComandoTerm::Ridimensiona(_, _) => {}
                ComandoTerm::Chiudi => break,
            }
        }
    });

    Ok(Canale {
        input: tx_in,
        output: rx_out,
    })
}
