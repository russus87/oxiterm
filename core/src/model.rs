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

/// Tipo di terminale di una sessione salvata.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TipoSessione {
    Ssh,
    Locale,
    Telnet,
    Seriale,
    Vnc,
}

/// Configurazione di un host intermedio (bastion / ProxyJump) per arrivare al
/// server vero attraverso un'altra macchina SSH.
#[derive(Clone, Serialize, Deserialize)]
pub struct JumpConfig {
    pub host: String,
    pub porta: u16,
    pub utente: String,
    pub auth: Auth,
}

impl Default for TipoSessione {
    fn default() -> Self {
        TipoSessione::Ssh
    }
}

/// Una sessione salvata dall'utente (la rubrica del "session manager").
/// Volutamente NON contiene password: solo i dati di connessione.
#[derive(Clone, Serialize, Deserialize)]
pub struct Sessione {
    pub id: String,
    pub nome: String,
    #[serde(default)]
    pub tipo: TipoSessione,
    #[serde(default)]
    pub host: String,
    #[serde(default = "porta_ssh")]
    pub porta: u16,
    #[serde(default)]
    pub utente: String,
    /// Percorso a una chiave privata da preferire, se presente.
    #[serde(default)]
    pub chiave: Option<String>,
    /// Cartella/gruppo di appartenenza nella rubrica.
    #[serde(default)]
    pub gruppo: Option<String>,
    /// Colore dell'etichetta (CSS), per distinguere a colpo d'occhio.
    #[serde(default)]
    pub colore: Option<String>,
    /// Solo per seriale: porta (COM3, /dev/ttyUSB0) e baud rate.
    #[serde(default)]
    pub porta_seriale: Option<String>,
    #[serde(default)]
    pub baud: Option<u32>,
    /// Host intermedio (jump/bastion) opzionale, senza segreti.
    #[serde(default)]
    pub jump_host: Option<String>,
    #[serde(default)]
    pub jump_porta: Option<u16>,
    #[serde(default)]
    pub jump_utente: Option<String>,
    #[serde(default)]
    pub jump_chiave: Option<String>,
}

fn porta_ssh() -> u16 {
    22
}

/// Una voce mostrata nel browser SFTP (file o cartella).
#[derive(Clone, Serialize, Deserialize)]
pub struct VoceFile {
    pub nome: String,
    pub dir: bool,
    pub dimensione: u64,
}

/// Un comando salvato (snippet/macro) da inviare al terminale con un clic.
#[derive(Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub id: String,
    pub nome: String,
    pub comando: String,
}
