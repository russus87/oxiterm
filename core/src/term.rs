//! Tipi comuni a tutti i terminali (SSH, locale, Telnet, seriale).
//!
//! L'idea: qualunque sia il trasporto, un terminale è una coppia di canali.
//! - `input`  comandi dalla UI verso il processo/connessione (tasti, resize, chiudi)
//! - `output` byte prodotti dal processo/connessione da mostrare nel terminale
//!
//! Così il livello desktop tratta ogni sessione allo stesso modo.

use tokio::sync::mpsc;

/// Comando inviato a un terminale in esecuzione.
pub enum ComandoTerm {
    /// Byte digitati dall'utente.
    Scrivi(Vec<u8>),
    /// Nuova dimensione (colonne, righe). Ignorato dai trasporti senza PTY.
    Ridimensiona(u32, u32),
    /// Chiudi il terminale.
    Chiudi,
}

/// Estremità di un terminale: si scrive su `input`, si legge da `output`.
pub struct Canale {
    pub input: mpsc::Sender<ComandoTerm>,
    pub output: mpsc::Receiver<Vec<u8>>,
}
