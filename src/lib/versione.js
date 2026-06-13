// Versione dell'app e changelog, mostrati nel pannello Info.
export const VERSIONE = "0.3.0";

export const changelog = [
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
