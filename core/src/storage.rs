//! Persistenza delle sessioni salvate (il "session manager"): un semplice
//! file JSON con la rubrica dei server. Nessuna password viene mai salvata.

use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::model::{Sessione, Snippet};

/// Legge una lista JSON da file; se manca o è illeggibile, restituisce vuoto.
fn carica<T: DeserializeOwned>(file: &Path) -> Vec<T> {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Salva una lista come JSON leggibile, creando la cartella se serve.
fn salva<T: Serialize>(file: &Path, dati: &[T]) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let testo = serde_json::to_string_pretty(dati).map_err(|e| e.to_string())?;
    std::fs::write(file, testo).map_err(|e| e.to_string())
}

/// Rubrica delle sessioni.
pub fn carica_sessioni(file: &Path) -> Vec<Sessione> {
    carica(file)
}
pub fn salva_sessioni(file: &Path, sessioni: &[Sessione]) -> Result<(), String> {
    salva(file, sessioni)
}

/// Libreria degli snippet/macro.
pub fn carica_snippet(file: &Path) -> Vec<Snippet> {
    carica(file)
}
pub fn salva_snippet(file: &Path, snippet: &[Snippet]) -> Result<(), String> {
    salva(file, snippet)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::model::{Sessione, TipoSessione};

    #[test]
    fn salva_e_ricarica() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sessioni.json");
        assert!(carica_sessioni(&file).is_empty());

        let s = Sessione {
            id: "1".into(),
            nome: "prova".into(),
            tipo: TipoSessione::Ssh,
            host: "esempio.com".into(),
            porta: 22,
            utente: "root".into(),
            chiave: None,
            gruppo: Some("g".into()),
            colore: None,
            porta_seriale: None,
            baud: None,
            tags: vec!["a".into(), "b".into()],
            comandi_avvio: None,
            jump_host: None,
            jump_porta: None,
            jump_utente: None,
            jump_chiave: None,
        };
        salva_sessioni(&file, &[s]).unwrap();

        let ricaricate = carica_sessioni(&file);
        assert_eq!(ricaricate.len(), 1);
        assert_eq!(ricaricate[0].nome, "prova");
        assert_eq!(ricaricate[0].tags, vec!["a", "b"]);
    }
}
