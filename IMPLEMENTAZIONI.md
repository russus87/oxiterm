# Diario delle implementazioni — Oxiterm

Registro di tutto ciò che viene implementato, fase per fase. Ogni fase è compilata
(`cargo check --workspace`) e il frontend è costruito (`npm run build`) prima di passare oltre.

Legenda: ✅ fatto · 🚧 in corso · ⏳ pianificato · ❌ fuori scope

---

## Fase 1 — SSH a schede + SFTP ✅ (base già esistente)

- ✅ Connessioni SSH multiple a schede, terminale xterm.js con PTY
- ✅ Pannello SFTP affiancato (lista/scarica/carica/mkdir/rinomina/elimina)
- ✅ Session manager (rubrica salvata, senza password)
- ✅ Auth password o chiave privata
- ✅ Workflow GitHub Actions per pacchetti Win/Mac/Linux

---

## Fase 2 — Sicurezza + nuovi tipi di connessione ✅

- ✅ Verifica `known_hosts` (TOFU: `core/ssh.rs`, file `known_hosts.json`, rifiuta se la chiave cambia)
- ✅ Terminale **locale** (PTY della shell di sistema) — `core/locale.rs` (portable-pty)
- ✅ **Telnet** con negoziazione IAC minimale — `core/telnet.rs`
- ✅ **Seriale** (porta + baud, elenco porte) — `core/seriale.rs` (serialport)
- ✅ Backend generalizzato: tipo unico `Canale`/`ComandoTerm` in `core/term.rs`;
  comandi `term_scrivi/ridimensiona/chiudi` validi per tutti i trasporti

## Fase 3 — Tunnel SSH (port forwarding) ✅

- ✅ Forward **locale** (-L): porta locale → host:porta remoti via SSH
- ✅ Proxy **dinamico SOCKS5** (-D) implementato a mano in `core/ssh.rs`
- ✅ UI per gestire i tunnel (`components/Tunnel.svelte`): crea/elenca/ferma
- ⏳ Forward **remoto** (-R) — più complesso, rimandato

## Fase 4 — Esperienza terminale ✅

- ✅ Impostazioni persistite (`lib/impostazioni.svelte.js`): font, dimensione, tema, cursore, scrollback
- ✅ Temi/schemi colore (`lib/temi.js`): Scuro, Solarized, Dracula, Gruvbox, Chiaro
- ✅ Ricerca nel terminale (addon search, Ctrl+Shift+F)
- ✅ Link cliccabili (addon web-links)
- ✅ **Broadcast input**: invia l'input a tutte le schede (`Impostazioni`, indicatore in tabbar)
- ✅ Copia/incolla: gestiti dai comportamenti predefiniti di xterm

## Fase 5 — Session manager avanzato ✅

- ✅ Cartelle/gruppi di sessioni (campo `gruppo`, sidebar raggruppata)
- ✅ Colore per sessione (selettore nel form, mostrato nella rubrica)

## Fase 6 — SFTP avanzato ✅

- ✅ Barra del percorso modificabile (Invio per navigare) + breadcrumb cliccabile
- ✅ Indicatore di stato durante i trasferimenti (caricamento/scaricamento)

## Fase 7 — Snippet / macro ✅

- ✅ Libreria di comandi salvati (`components/Snippet.svelte`, backend `storage.rs`),
  inviabili al terminale attivo con un clic

## Extra ✅

- ✅ Keepalive SSH (15s) per connessioni stabili
- ✅ Azioni per scheda: riconnetti, duplica, pulisci, zoom (A+/A−)
- ✅ Import/Export della rubrica sessioni (comandi backend + UI)
- ✅ Pannello Info con versione (0.2.0) e changelog

---

## Stato build (verificato)

- `cargo check --workspace` ✅
- `cargo build -p oxiterm` ✅ (binario `target/debug/oxiterm` prodotto, ~241 MB debug)
- `npm run build` ✅
- GUI non avviata in automatico (richiede display): da provare con `cargo tauri dev`

## Da fare / idee future

- Forward remoto SSH (-R)
- Apertura file remoti in un editor (download temporaneo + ricarica)
- Verifica known_hosts interattiva (oggi: rifiuto automatico se la chiave cambia)
- Split dei pannelli (più terminali nella stessa scheda)
- Client RDP/VNC

## Fuori scope (non clonabili realisticamente)

- ❌ Server X11 integrato
- ❌ Mosh
