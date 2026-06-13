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

## Roadmap

- [ ] Verifica `known_hosts` (oggi accetta qualsiasi chiave del server)
- [ ] Tunnel / port forwarding SSH
- [ ] Terminale locale (PTY) e Telnet
- [ ] Seriale
- [ ] Macro e split dei pannelli
- [ ] Client RDP/VNC

## Licenza

MIT
