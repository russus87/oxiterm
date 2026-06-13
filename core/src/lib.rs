//! Cuore di Oxiterm: logica SSH/SFTP riutilizzabile, senza dipendenze da Tauri.
//! - `model`   tipi dati condivisi col frontend (sessioni, auth, voci file)
//! - `ssh`     connessione SSH e shell interattiva (PTY)
//! - `sftp`    operazioni sul filesystem remoto
//! - `storage` salvataggio/lettura delle sessioni salvate (session manager)

pub mod model;
pub mod sftp;
pub mod ssh;
pub mod storage;
