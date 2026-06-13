//! Cuore di Oxiterm: logica riutilizzabile, senza dipendenze da Tauri.
//! - `model`   tipi dati condivisi col frontend
//! - `term`    tipi comuni a tutti i terminali (canale input/output)
//! - `ssh`     connessione SSH: shell (PTY), SFTP, tunnel, known_hosts
//! - `sftp`    operazioni sul filesystem remoto
//! - `locale`  terminale locale (PTY della shell di sistema)
//! - `telnet`  client Telnet minimale
//! - `seriale` console seriale (porte COM / tty)
//! - `storage` salvataggio/lettura delle sessioni salvate (session manager)

pub mod locale;
pub mod model;
pub mod seriale;
pub mod sftp;
pub mod ssh;
pub mod storage;
pub mod telnet;
pub mod term;
pub mod vnc;
