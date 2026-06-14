// Versione dell'app e changelog, mostrati nel pannello Info.
export const VERSIONE = "0.7.0";

export const changelog = [
  {
    versione: "0.7.0",
    note: [
      "Strumenti di rete: ping, traceroute, controllo porta, Wake-on-LAN",
      "Monitor server (uptime/memoria/disco/processi) con aggiornamento live",
      "VNC: cursore remoto disegnato correttamente",
    ],
  },
  {
    versione: "0.6.1",
    note: [
      "Repo sorgente pubblico: auto-update direttamente dalle release di GitHub",
      "Rimosso il repo dist separato (semplificazione)",
    ],
  },
  {
    versione: "0.6.0",
    note: [
      "Import da ~/.ssh/config",
      "Quick connect (utente@host:porta)",
      "Comandi automatici all'avvio della sessione",
      "Anteprima file SFTP (testo e immagini)",
      "Tema dell'interfaccia chiaro/scuro + scala UI",
      "Notifiche desktop (sessione caduta, trasferimenti)",
      "Riordino schede col drag e ripristino sessioni all'avvio",
      "Segnalibri SFTP sincronizzati col cloud",
      "Auto-aggiornamento (artefatti firmati sul repo pubblico oxiterm-dist)",
    ],
  },
  {
    versione: "0.5.3",
    note: [
      "Fix: 'Connetti' non faceva nulla nel pacchetto (crypto.randomUUID assente nella webview)",
      "Nuovo pulsante 'Salva' nel form: salva la sessione senza connettersi",
    ],
  },
  {
    versione: "0.5.2",
    note: [
      "Fix pacchetto Arch: compilato in modalità produzione (non cerca più localhost)",
    ],
  },
  {
    versione: "0.5.1",
    note: ["Pacchetto Arch Linux (.pkg.tar.zst) generato dalla CI"],
  },
  {
    versione: "0.5.0",
    note: [
      "Indicatore di stato connessione (pallino verde/giallo/rosso) per scheda",
      "Gestione chiavi SSH: genera, elenca e copia sul server (ssh-copy-id)",
      "Editor di testo remoto integrato (oltre all'editor di sistema)",
      "Segnalibri SFTP e trasferimento ricorsivo di cartelle",
      "Tag per le sessioni (ricercabili nella palette)",
      "Registrazione e replay delle sessioni (formato asciicast)",
      "Vault cifrato per le password salvate (master password)",
      "Test automatici del core",
    ],
  },
  {
    versione: "0.4.0",
    note: [
      "Riconnessione automatica delle sessioni cadute (con attesa crescente)",
      "Registrazione della sessione su file (log)",
      "Colore per scheda (dalla rubrica) per riconoscere gli host a colpo d'occhio",
      "Palette comandi (Ctrl+P) per aprire al volo le sessioni salvate",
      "Drag & drop di file nel pannello SFTP + coda con barre di avanzamento",
      "Jump host / ProxyJump: connessione attraverso un bastion",
      "Sincronizzazione cloud della rubrica via Git (push/pull)",
      "Client VNC sperimentale (canvas + mouse/tastiera)",
    ],
  },
  {
    versione: "0.3.0",
    note: [
      "Forward remoto SSH (-R) oltre a locale (-L) e SOCKS5 (-D)",
      "known_hosts interattivo: conferma chiave nuova o cambiata",
      "Apertura file remoti nell'editor di sistema con auto-salvataggio",
      "Split dei pannelli: più terminali affiancati o impilati nella stessa scheda",
    ],
  },
  {
    versione: "0.2.0",
    note: [
      "Terminale locale, Telnet e seriale oltre all'SSH",
      "Tunnel SSH: forward locale (-L) e proxy SOCKS5 (-D)",
      "Verifica known_hosts, keepalive SSH",
      "Impostazioni e temi colore, ricerca, link cliccabili, broadcast",
      "Gruppi e colori nella rubrica; snippet/macro",
      "SFTP con breadcrumb, percorso modificabile e stato trasferimenti",
      "Azioni scheda: riconnetti, duplica, pulisci, zoom",
      "Import/Export della rubrica",
    ],
  },
  {
    versione: "0.1.0",
    note: ["Prima versione: SSH a schede + pannello SFTP + session manager"],
  },
];
