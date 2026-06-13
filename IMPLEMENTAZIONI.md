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

## Fase 8 — completamento roadmap ✅ (v0.3.0)

- ✅ **Forward remoto SSH (-R)**: il server ascolta e inoltra verso un bersaglio locale
  (`core/ssh.rs`: `tunnel_remoto` + handler `server_channel_open_forwarded_tcpip`)
- ✅ **known_hosts interattivo**: alla prima connessione o se la chiave cambia, la UI mostra
  l'impronta SHA256 e chiede conferma; ritenta con "modo di fiducia" (errore `HOSTKEY:` dal core)
- ✅ **Apertura file remoti in editor**: `sftp_apri_editor` scarica in temp, apre con l'app di
  sistema (plugin opener) e **ricarica automaticamente** sul server a ogni salvataggio (polling mtime)
- ✅ **Split dei pannelli**: una scheda può contenere fino a 4 terminali affiancati (▦) o impilati (▤),
  ognuno con la propria connessione; chiusura del singolo pannello

## Fase 9 — i 9 punti proposti ✅ (v0.4.0)

1. ✅ **Riconnessione automatica**: backoff esponenziale 1→30s (`Terminale.svelte`), opzione in Impostazioni
2. ✅ **Log di sessione su file**: `term_log_avvia/ferma`, l'output viene scritto dal task di inoltro;
   pulsante "⏺ Log" nella tabbar
3. ✅ **Colore per host**: campo `colore` applicato a scheda (bordo) e rubrica
4. ✅ **Palette comandi (Ctrl+P)**: `PaletteComandi.svelte`, ricerca + frecce + Invio
5. ✅ **Drag & drop SFTP**: `getCurrentWebview().onDragDropEvent` nel pannello attivo → upload
6. ✅ **Coda trasferimenti con avanzamento**: `sftp::carica/scarica_progresso` + eventi `sftp-progresso`,
   store `trasferimenti.svelte.js`, `CodaTrasferimenti.svelte` con barre
7. ✅ **Client VNC (sperimentale)**: `core/vnc.rs` (crate `vnc`), `VncView.svelte` (canvas + mouse/tastiera),
   tipo sessione "vnc". ⚠️ Non testato dal vivo: da verificare/rifinire.
8. ✅ **Sincronizzazione cloud (Git)**: `src-tauri/git_sync.rs` (push/pull con git2), UI in Impostazioni
9. ✅ **Jump host / ProxyJump**: `connect_stream` attraverso un bastion (`core/ssh.rs`), campi nel form

## Non implementati — valutazione onesta

- ❌ **Client RDP / VNC**: sono di fatto applicazioni a sé (rendering del desktop remoto frame per
  frame su canvas + inoltro input). Fattibili in Rust (IronRDP, vnc-rs) ma è una fase grossa e
  rischiosa da consegnare non testata. Proposti come progetto dedicato a parte.
- ❌ **Mosh**: protocollo UDP (SSP) che richiede `mosh-server` sul remoto e una macchina a stati
  complessa; sproporzionato rispetto al valore qui.
- ❌ **Server X11**: escluso su tua indicazione.

## Fuori scope (non clonabili realisticamente)

- ❌ Server X11 integrato
- ❌ Mosh
