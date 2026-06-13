//! Sincronizzazione cloud della rubrica via Git.
//!
//! La cartella di configurazione (dove vivono sessioni.json/snippet.json/…) viene
//! gestita come un repository Git: `push` committa e carica, `pull` scarica e fa
//! fast-forward. Le credenziali vengono prese dall'agent SSH o dal credential
//! helper di sistema, così funziona con GitHub/GitLab senza salvare password.

use std::path::Path;

use git2::{Cred, FetchOptions, PushOptions, RemoteCallbacks, Repository, Signature};

/// Apre il repository nella cartella, creandolo se non esiste.
fn apri(dir: &Path) -> Result<Repository, String> {
    Repository::open(dir)
        .or_else(|_| Repository::init(dir))
        .map_err(|e| e.to_string())
}

/// Callback per le credenziali (SSH agent o credential helper).
fn callbacks() -> RemoteCallbacks<'static> {
    let mut cb = RemoteCallbacks::new();
    cb.credentials(|url, utente, permessi| {
        if permessi.contains(git2::CredentialType::SSH_KEY) {
            Cred::ssh_key_from_agent(utente.unwrap_or("git"))
        } else if permessi.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
            let cfg = git2::Config::open_default()?;
            Cred::credential_helper(&cfg, url, utente)
        } else {
            Cred::default()
        }
    });
    cb
}

/// URL del remote "origin", se configurato.
pub fn remote_url(dir: &Path) -> Option<String> {
    let repo = Repository::open(dir).ok()?;
    let remote = repo.find_remote("origin").ok()?;
    let url = remote.url().map(|s| s.to_string());
    url
}

/// Imposta (o aggiorna) l'URL del remote "origin".
pub fn imposta_remote(dir: &Path, url: &str) -> Result<(), String> {
    let repo = apri(dir)?;
    if repo.find_remote("origin").is_ok() {
        repo.remote_set_url("origin", url).map_err(|e| e.to_string())?;
    } else {
        repo.remote("origin", url).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Committa i file di configurazione e li carica sul remote.
pub fn push(dir: &Path) -> Result<(), String> {
    let repo = apri(dir)?;

    // Mette in staging tutti i .json della cartella.
    let mut index = repo.index().map_err(|e| e.to_string())?;
    index
        .add_all(["*.json"].iter(), git2::IndexAddOption::DEFAULT, None)
        .map_err(|e| e.to_string())?;
    index.write().map_err(|e| e.to_string())?;
    let albero_id = index.write_tree().map_err(|e| e.to_string())?;
    let albero = repo.find_tree(albero_id).map_err(|e| e.to_string())?;

    let firma = Signature::now("Oxiterm", "oxiterm@local").map_err(|e| e.to_string())?;
    let genitore = repo
        .head()
        .ok()
        .and_then(|h| h.target())
        .and_then(|oid| repo.find_commit(oid).ok());
    let genitori: Vec<&git2::Commit> = genitore.iter().collect();
    repo.commit(Some("HEAD"), &firma, &firma, "sync", &albero, &genitori)
        .map_err(|e| e.to_string())?;

    let ramo = nome_ramo(&repo);
    let mut remote = repo.find_remote("origin").map_err(|e| e.to_string())?;
    let mut po = PushOptions::new();
    po.remote_callbacks(callbacks());
    remote
        .push(
            &[format!("refs/heads/{ramo}:refs/heads/{ramo}")],
            Some(&mut po),
        )
        .map_err(|e| e.to_string())
}

/// Scarica dal remote e fa fast-forward (errore se i due lati sono divergenti).
pub fn pull(dir: &Path) -> Result<(), String> {
    let repo = apri(dir)?;
    let ramo = nome_ramo(&repo);

    let mut fo = FetchOptions::new();
    fo.remote_callbacks(callbacks());
    let mut remote = repo.find_remote("origin").map_err(|e| e.to_string())?;
    remote
        .fetch(&[&ramo], Some(&mut fo), None)
        .map_err(|e| e.to_string())?;

    let fetch_head = repo.find_reference("FETCH_HEAD").map_err(|e| e.to_string())?;
    let commit = repo
        .reference_to_annotated_commit(&fetch_head)
        .map_err(|e| e.to_string())?;
    let (analisi, _) = repo.merge_analysis(&[&commit]).map_err(|e| e.to_string())?;

    if analisi.is_up_to_date() {
        return Ok(());
    }
    if analisi.is_fast_forward() || repo.head().is_err() {
        // Sposta il ramo al commit remoto e aggiorna i file in cartella.
        let nome_rif = format!("refs/heads/{ramo}");
        let oid = commit.id();
        match repo.find_reference(&nome_rif) {
            Ok(mut r) => {
                r.set_target(oid, "fast-forward").map_err(|e| e.to_string())?;
            }
            Err(_) => {
                repo.reference(&nome_rif, oid, true, "init")
                    .map_err(|e| e.to_string())?;
            }
        }
        repo.set_head(&nome_rif).map_err(|e| e.to_string())?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("le modifiche locali e remote sono divergenti: fai prima 'Push'".into())
    }
}

/// Nome del ramo corrente (default "main").
fn nome_ramo(repo: &Repository) -> String {
    repo.head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_string()))
        .unwrap_or_else(|| "main".into())
}
