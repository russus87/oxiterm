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
    modo: p.modo ?? null, // null=normale, "accetta", "sostituisci"
    jump: p.jump ?? null, // { host, porta, utente, auth } oppure null
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
export const termLogAvvia = (id, percorso) => invoke("term_log_avvia", { id, percorso });
export const termLogFerma = (id) => invoke("term_log_ferma", { id });
export const termRecAvvia = (id, percorso) => invoke("term_rec_avvia", { id, percorso });
export const termRecFerma = (id) => invoke("term_rec_ferma", { id });
export const leggiFile = (percorso) => invoke("leggi_file", { percorso });

// ---- Chiavi SSH ----
export const generaChiave = (nome, commento) =>
  invoke("genera_chiave", { nome, commento });
export const listaChiavi = () => invoke("lista_chiavi");
export const copiaChiave = (id, pubblica) => invoke("copia_chiave", { id, pubblica });

// ---- Vault cifrato ----
export const vaultStato = () => invoke("vault_stato");
export const vaultSblocca = (master) => invoke("vault_sblocca", { master });
export const vaultBlocca = () => invoke("vault_blocca");
export const vaultSalvaPassword = (id, password) =>
  invoke("vault_salva_password", { id, password });
export const vaultLeggiPassword = (id) => invoke("vault_leggi_password", { id });

// ---- VNC (sperimentale) ----
export const apriVnc = (id, host, porta, password) =>
  invoke("apri_vnc", { id, host, porta, password: password || null });
export const vncMouse = (id, x, y, bottoni) =>
  invoke("vnc_mouse", { id, x, y, bottoni });
export const vncTasto = (id, giu, key) => invoke("vnc_tasto", { id, giu, key });
export const vncChiudi = (id) => invoke("vnc_chiudi", { id });

// ---- Strumenti di rete + monitor ----
export const netPorta = (host, porta) => invoke("net_porta", { host, porta });
export const netPing = (host) => invoke("net_ping", { host });
export const netTraceroute = (host) => invoke("net_traceroute", { host });
export const netWol = (mac) => invoke("net_wol", { mac });
export const serverStato = (id) => invoke("server_stato", { id });

// ---- Tunnel SSH ----
export const tunnelLocale = (id, portaLocale, hostRemoto, portaRemota) =>
  invoke("tunnel_locale", { id, portaLocale, hostRemoto, portaRemota });
export const tunnelSocks = (id, portaLocale) =>
  invoke("tunnel_socks", { id, portaLocale });
export const tunnelRemoto = (id, portaRemota, hostLocale, portaLocale) =>
  invoke("tunnel_remoto", { id, portaRemota, hostLocale, portaLocale });
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
export const sftpApriEditor = (id, remoto) => invoke("sftp_apri_editor", { id, remoto });
export const sftpLeggiTesto = (id, remoto) => invoke("sftp_leggi_testo", { id, remoto });
export const sftpScriviTesto = (id, remoto, contenuto) =>
  invoke("sftp_scrivi_testo", { id, remoto, contenuto });
export const sftpCaricaCartella = (id, locale, remoto) =>
  invoke("sftp_carica_cartella", { id, locale, remoto });
export const sftpScaricaCartella = (id, remoto, locale) =>
  invoke("sftp_scarica_cartella", { id, remoto, locale });
export const sftpLeggiBase64 = (id, remoto) => invoke("sftp_leggi_base64", { id, remoto });
export const sftpCaricaCoda = (id, trasferimento, locale, remoto) =>
  invoke("sftp_carica_coda", { id, trasferimento, locale, remoto });
export const sftpScaricaCoda = (id, trasferimento, remoto, locale) =>
  invoke("sftp_scarica_coda", { id, trasferimento, remoto, locale });

// ---- Session manager (rubrica salvata) ----
export const listaSessioni = () => invoke("lista_sessioni");
export const salvaSessione = (sessione) => invoke("salva_sessione", { sessione });
export const eliminaSessione = (id) => invoke("elimina_sessione", { id });
export const esportaRubrica = (percorso) => invoke("esporta_rubrica", { percorso });
export const importaRubrica = (percorso) => invoke("importa_rubrica", { percorso });
export const importaSshConfig = () => invoke("importa_ssh_config");

// ---- Segnalibri SFTP (sincronizzabili) ----
export const listaSegnalibri = () => invoke("lista_segnalibri");
export const salvaSegnalibri = (lista) => invoke("salva_segnalibri", { lista });

// ---- Sincronizzazione cloud (Git) ----
export const syncRemote = () => invoke("sync_remote");
export const syncImpostaRemote = (url) => invoke("sync_imposta_remote", { url });
export const syncPush = () => invoke("sync_push");
export const syncPull = () => invoke("sync_pull");

// ---- Snippet / macro ----
export const listaSnippet = () => invoke("lista_snippet");
export const salvaSnippet = (snippet) => invoke("salva_snippet", { snippet });
export const eliminaSnippet = (id) => invoke("elimina_snippet", { id });
