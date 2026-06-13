//! Persistenza delle sessioni salvate (il "session manager"): un semplice
//! file JSON con la rubrica dei server. Nessuna password viene mai salvata.

use std::path::Path;

use crate::model::Sessione;

/// Legge le sessioni salvate; se il file non esiste restituisce una lista vuota.
pub fn carica_sessioni(file: &Path) -> Vec<Sessione> {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Salva l'intera rubrica delle sessioni (JSON leggibile).
pub fn salva_sessioni(file: &Path, sessioni: &[Sessione]) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let testo = serde_json::to_string_pretty(sessioni).map_err(|e| e.to_string())?;
    std::fs::write(file, testo).map_err(|e| e.to_string())
}
