# Oxiterm 🦀

Terminale SSH/SFTP a schede, in stile **MobaXterm**, scritto in **Rust + Tauri v2**
con frontend **Svelte 5**. Pensato per essere pacchettizzato per Windows, macOS e Linux.

## Funzionalità (Fase 1)

- **Connessioni SSH a schede** con terminale completo (xterm.js + PTY).
- **Pannello SFTP** affiancato al terminale: sfoglia, scarica, carica, crea cartelle,
  rinomina ed elimina file remoti.
- **Session manager**: rubrica delle connessioni salvate (host/porta/utente, niente
  password su disco).
- Autenticazione con **password** o **chiave privata** (con passphrase).

## Architettura

Stesso schema del progetto *rustman*: workspace Cargo con il `core` puro Rust separato
dal livello desktop.

```
core/        logica riutilizzabile, senza Tauri
  ssh.rs       connessione SSH + shell con PTY (russh)
  sftp.rs      operazioni sul filesystem remoto (russh-sftp)
  storage.rs   rubrica delle sessioni salvate (JSON)
  model.rs     tipi condivisi col frontend
src-tauri/   app desktop: comandi Tauri che chiamano il core
src/         frontend Svelte 5
  components/Terminale.svelte   terminale di una sessione
  components/Sftp.svelte        pannello file remoti
  App.svelte                    sidebar + schede
```

## Sviluppo

```bash
npm install
cargo tauri dev      # avvia l'app desktop
```

## Pacchetti

Spingendo un tag `vX.Y.Z` la GitHub Action `release` compila e pubblica
i pacchetti per le tre piattaforme.

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Funzionalità (dalla v0.2.0)

- [x] Verifica `known_hosts` (TOFU) + keepalive SSH
- [x] Tunnel / port forwarding SSH (locale -L e SOCKS5 dinamico -D)
- [x] Terminale locale (PTY) e Telnet
- [x] Seriale
- [x] Snippet/macro, broadcast input, ricerca, temi colore, impostazioni
- [x] Gruppi e colori nella rubrica; import/export
- [x] SFTP con breadcrumb, percorso modificabile, stato trasferimenti
- [x] Azioni scheda: riconnetti, duplica, pulisci, zoom

- [x] Forward remoto SSH (-R), known_hosts interattivo
- [x] Apertura file remoti in editor con auto-salvataggio
- [x] Split dei pannelli (fino a 4 terminali per scheda)
- [x] Riconnessione automatica, log di sessione, colore per host
- [x] Palette comandi (Ctrl+P), drag&drop + coda trasferimenti SFTP
- [x] Jump host / ProxyJump, sincronizzazione cloud della rubrica (Git)
- [x] Client VNC (sperimentale)
- [x] Stato connessione, gestione chiavi SSH, editor remoto integrato
- [x] Segnalibri SFTP, trasferimento ricorsivo, tag sessioni
- [x] Registrazione & replay (asciicast), vault cifrato (master password)
- [x] Test automatici del core

## Roadmap

- [ ] Rifinitura/test del client VNC
- [ ] Client RDP (progetto a sé)
- [ ] Mosh

Vedi `IMPLEMENTAZIONI.md` per il diario dettagliato di tutto ciò che è stato fatto.

## Licenza

MIT
