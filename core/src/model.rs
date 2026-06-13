//! Tipi dati condivisi tra backend e frontend (serializzati in JSON).

use serde::{Deserialize, Serialize};

/// Come autenticarsi su un server SSH. Il segreto arriva dal frontend solo
/// al momento della connessione: non viene mai salvato su disco.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "lowercase")]
pub enum Auth {
    /// Autenticazione con password.
    Password { password: String },
    /// Autenticazione con chiave privata (percorso al file + passphrase opzionale).
    Chiave {
        percorso: String,
        passphrase: Option<String>,
    },
}

/// Una sessione salvata dall'utente (la rubrica del "session manager").
/// Volutamente NON contiene password: solo i dati di connessione.
#[derive(Clone, Serialize, Deserialize)]
pub struct Sessione {
    pub id: String,
    pub nome: String,
    pub host: String,
    pub porta: u16,
    pub utente: String,
    /// Percorso a una chiave privata da preferire, se presente.
    #[serde(default)]
    pub chiave: Option<String>,
}

/// Una voce mostrata nel browser SFTP (file o cartella).
#[derive(Clone, Serialize, Deserialize)]
pub struct VoceFile {
    pub nome: String,
    pub dir: bool,
    pub dimensione: u64,
}
