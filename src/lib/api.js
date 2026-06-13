// Ponte verso il backend Tauri: ogni funzione qui sotto richiama un comando
// Rust via `invoke`. La UI usa solo queste funzioni, mai `invoke` direttamente.
import { invoke } from "@tauri-apps/api/core";

// ---- SSH / terminale ----
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
export const sshScrivi = (id, dati) => invoke("ssh_scrivi", { id, dati });
export const sshRidimensiona = (id, colonne, righe) =>
  invoke("ssh_ridimensiona", { id, colonne, righe });
export const sshDisconnetti = (id) => invoke("ssh_disconnetti", { id });

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
