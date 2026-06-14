//! Parser minimale di ~/.ssh/config: estrae gli host per popolare la rubrica.
//! Riconosce HostName, User, Port, IdentityFile e ProxyJump. Gli Host con
//! caratteri jolly (*, ?) vengono ignorati (non sono host concreti).

use std::collections::HashMap;

use crate::model::{Sessione, TipoSessione};

/// Converte il testo di un ssh_config in una lista di sessioni.
pub fn parse(testo: &str) -> Vec<Sessione> {
    let mut out = Vec::new();
    let mut nomi: Vec<String> = Vec::new();
    let mut campi: HashMap<String, String> = HashMap::new();

    for riga in testo.lines() {
        let riga = riga.trim();
        if riga.is_empty() || riga.starts_with('#') {
            continue;
        }
        // "Keyword valore" (anche con '=').
        let (chiave, valore) = match riga.split_once(|c: char| c == ' ' || c == '\t' || c == '=') {
            Some((k, v)) => (k.trim().to_lowercase(), v.trim().trim_start_matches('=').trim().to_string()),
            None => continue,
        };

        if chiave == "host" {
            // Chiude il blocco precedente e ne apre uno nuovo.
            flush(&mut out, &nomi, &campi);
            campi.clear();
            nomi = valore.split_whitespace().map(|s| s.to_string()).collect();
        } else {
            campi.insert(chiave, valore);
        }
    }
    flush(&mut out, &nomi, &campi);
    out
}

fn flush(out: &mut Vec<Sessione>, nomi: &[String], campi: &HashMap<String, String>) {
    for nome in nomi {
        if nome.contains('*') || nome.contains('?') {
            continue;
        }
        let (jh, jp, ju) = parse_jump(campi.get("proxyjump"));
        out.push(Sessione {
            id: format!("sshcfg-{nome}"),
            nome: nome.clone(),
            tipo: TipoSessione::Ssh,
            host: campi.get("hostname").cloned().unwrap_or_else(|| nome.clone()),
            porta: campi
                .get("port")
                .and_then(|p| p.parse().ok())
                .unwrap_or(22),
            utente: campi.get("user").cloned().unwrap_or_default(),
            chiave: campi.get("identityfile").cloned(),
            gruppo: Some("~/.ssh/config".into()),
            colore: None,
            porta_seriale: None,
            baud: None,
            tags: Vec::new(),
            comandi_avvio: None,
            jump_host: jh,
            jump_porta: jp,
            jump_utente: ju,
            jump_chiave: None,
        });
    }
}

/// Estrae host/porta/utente da un valore ProxyJump tipo "user@host:porta".
fn parse_jump(v: Option<&String>) -> (Option<String>, Option<u16>, Option<String>) {
    let Some(v) = v else {
        return (None, None, None);
    };
    let v = v.split(',').next().unwrap_or(v).trim(); // prendi solo il primo hop
    if v.is_empty() {
        return (None, None, None);
    }
    let (utente, resto) = match v.split_once('@') {
        Some((u, r)) => (Some(u.to_string()), r),
        None => (None, v),
    };
    let (host, porta) = match resto.split_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().ok()),
        None => (resto.to_string(), None),
    };
    (Some(host), porta, utente)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_blocco_base() {
        let cfg = "
Host server1
    HostName 10.0.0.5
    User root
    Port 2222
    IdentityFile ~/.ssh/id_ed25519

Host *
    ServerAliveInterval 60
";
        let s = parse(cfg);
        assert_eq!(s.len(), 1); // l'Host * viene ignorato
        assert_eq!(s[0].nome, "server1");
        assert_eq!(s[0].host, "10.0.0.5");
        assert_eq!(s[0].utente, "root");
        assert_eq!(s[0].porta, 2222);
        assert_eq!(s[0].chiave.as_deref(), Some("~/.ssh/id_ed25519"));
    }

    #[test]
    fn parse_proxyjump() {
        let cfg = "Host interno\n  HostName 192.168.1.2\n  ProxyJump admin@bastion:2200\n";
        let s = parse(cfg);
        assert_eq!(s[0].jump_host.as_deref(), Some("bastion"));
        assert_eq!(s[0].jump_porta, Some(2200));
        assert_eq!(s[0].jump_utente.as_deref(), Some("admin"));
    }
}
