//! Terminale locale: avvia la shell di sistema in uno pseudo-terminale (PTY).
//! Usa `portable-pty`, che funziona su Linux, macOS e Windows.

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use tokio::sync::mpsc;

use crate::term::{Canale, ComandoTerm};

/// Apre un terminale locale con la shell indicata (o quella di default del SO).
pub fn apri(shell: Option<String>, colonne: u16, righe: u16) -> Result<Canale, String> {
    let sistema = native_pty_system();
    let coppia = sistema
        .openpty(PtySize {
            rows: righe,
            cols: colonne,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    // Shell di default in base al sistema operativo.
    let comando = shell.unwrap_or_else(shell_default);
    let cmd = CommandBuilder::new(comando);
    let mut figlio = coppia
        .slave
        .spawn_command(cmd)
        .map_err(|e| e.to_string())?;
    drop(coppia.slave); // non ci serve più il lato slave

    let mut lettore = coppia.master.try_clone_reader().map_err(|e| e.to_string())?;
    let mut scrittore = coppia.master.take_writer().map_err(|e| e.to_string())?;
    let master = coppia.master;

    let (tx_in, mut rx_in) = mpsc::channel::<ComandoTerm>(64);
    let (tx_out, rx_out) = mpsc::channel::<Vec<u8>>(256);

    // Thread bloccante che legge l'output del PTY e lo inoltra.
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match lettore.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if tx_out.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Task che gestisce input/resize/chiusura.
    tokio::spawn(async move {
        use std::io::Write;
        while let Some(cmd) = rx_in.recv().await {
            match cmd {
                ComandoTerm::Scrivi(b) => {
                    let _ = scrittore.write_all(&b);
                    let _ = scrittore.flush();
                }
                ComandoTerm::Ridimensiona(c, r) => {
                    let _ = master.resize(PtySize {
                        rows: r as u16,
                        cols: c as u16,
                        pixel_width: 0,
                        pixel_height: 0,
                    });
                }
                ComandoTerm::Chiudi => {
                    let _ = figlio.kill();
                    break;
                }
            }
        }
    });

    Ok(Canale {
        input: tx_in,
        output: rx_out,
    })
}

/// Legge la lettura standard per la shell di default del sistema.
fn shell_default() -> String {
    if cfg!(windows) {
        std::env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".into())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into())
    }
}
