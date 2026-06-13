// Ponte verso il backend Tauri: ogni funzione qui sotto richiama un comando
// Rust via `invoke`. La UI usa solo queste funzioni, mai `invoke` direttamente.
import { invoke } from "@tauri-apps/api/core";

// ---- Apertura sessioni ----
export const sshConnetti = (p) =>
  invoke("ssh_connetti", {
    id: p.id,
    host: p.host,
    porta: p.porta,
    utente: p.utente,
    auth: p.auth,
    colonne: p.colonne,
    righe: p.righe,
  });
export const apriLocale = (id, shell, colonne, righe) =>
  invoke("apri_locale", { id, shell: shell || null, colonne, righe });
export const apriTelnet = (id, host, porta) =>
  invoke("apri_telnet", { id, host, porta });
export const apriSeriale = (id, porta, baud) =>
  invoke("apri_seriale", { id, porta, baud });
export const porteSeriali = () => invoke("porte_seriali");

// ---- Terminale (qualsiasi tipo) ----
export const termScrivi = (id, dati) => invoke("term_scrivi", { id, dati });
export const termRidimensiona = (id, colonne, righe) =>
  invoke("term_ridimensiona", { id, colonne, righe });
export const termChiudi = (id) => invoke("term_chiudi", { id });

// ---- Tunnel SSH ----
export const tunnelLocale = (id, portaLocale, hostRemoto, portaRemota) =>
  invoke("tunnel_locale", { id, portaLocale, hostRemoto, portaRemota });
export const tunnelSocks = (id, portaLocale) =>
  invoke("tunnel_socks", { id, portaLocale });
export const listaTunnel = (id) => invoke("lista_tunnel", { id });
export const fermaTunnel = (id, tunnelId) =>
  invoke("ferma_tunnel", { id, tunnelId });

// ---- SFTP ----
export const sftpHome = (id) => invoke("sftp_home", { id });
export const sftpLista = (id, percorso) => invoke("sftp_lista", { id, percorso });
export const sftpScarica = (id, remoto, locale) =>
  invoke("sftp_scarica", { id, remoto, locale });
export const sftpCarica = (id, locale, remoto) =>
  invoke("sftp_carica", { id, locale, remoto });
export const sftpCreaCartella = (id, percorso) =>
  invoke("sftp_crea_cartella", { id, percorso });
export const sftpElimina = (id, percorso, dir) =>
  invoke("sftp_elimina", { id, percorso, dir });
export const sftpRinomina = (id, da, a) => invoke("sftp_rinomina", { id, da, a });

// ---- Session manager (rubrica salvata) ----
export const listaSessioni = () => invoke("lista_sessioni");
export const salvaSessione = (sessione) => invoke("salva_sessione", { sessione });
export const eliminaSessione = (id) => invoke("elimina_sessione", { id });

// ---- Snippet / macro ----
export const listaSnippet = () => invoke("lista_snippet");
export const salvaSnippet = (snippet) => invoke("salva_snippet", { snippet });
export const eliminaSnippet = (id) => invoke("elimina_snippet", { id });
