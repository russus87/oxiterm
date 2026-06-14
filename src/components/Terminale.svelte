<script>
  // Terminale di una singola sessione (SSH, locale, Telnet o seriale).
  // Gestisce tutto il ciclo di vita: crea xterm, avvia la connessione giusta in
  // base al tipo, applica le impostazioni, inoltra input/output, ricerca e link.
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { SearchAddon } from "@xterm/addon-search";
  import { WebLinksAddon } from "@xterm/addon-web-links";
  import { listen } from "@tauri-apps/api/event";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import * as api from "../lib/api.js";
  import { impostazioni } from "../lib/impostazioni.svelte.js";
  import { temi } from "../lib/temi.js";
  import { notifica } from "../lib/notifiche.js";

  // Proprietà: dati del tab + callback verso App.
  // `invia` decide se mandare l'input solo a questa scheda o a tutte (broadcast).
  let { tab, attivo, invia, onConnesso, onErrore, onChiuso, onHostKey } = $props();

  let elemento;
  let term, fit, search;
  let off = [];
  let mostraCerca = $state(false);
  let testoCerca = $state("");
  let distrutto = false;
  let timerRiconn;
  let tentativi = 0;

  onMount(async () => {
    term = new Terminal({
      fontFamily: impostazioni.fontFamily,
      fontSize: impostazioni.fontSize,
      cursorBlink: impostazioni.cursorBlink,
      scrollback: impostazioni.scrollback,
      theme: temi[impostazioni.tema] ?? temi.Scuro,
      allowProposedApi: true,
    });
    fit = new FitAddon();
    search = new SearchAddon();
    term.loadAddon(fit);
    term.loadAddon(search);
    // I link cliccabili si aprono nel browser di sistema (non nella webview).
    term.loadAddon(new WebLinksAddon((_e, uri) => openUrl(uri)));
    term.open(elemento);
    fit.fit();

    // Output e chiusura per QUESTA sessione.
    off.push(
      await listen(`term-dati-${tab.id}`, (e) =>
        term.write(new Uint8Array(e.payload)),
      ),
    );
    off.push(
      await listen(`term-chiuso-${tab.id}`, () => {
        term.write("\r\n\x1b[31m[sessione chiusa]\x1b[0m\r\n");
        onChiuso?.();
        if (tab.tipo !== "locale" && !distrutto) {
          notifica("Connessione persa", tab.nome);
        }
        // Auto-riconnessione (non per il terminale locale, dove chiudere è voluto).
        if (impostazioni.autoReconnect && tab.tipo !== "locale" && !distrutto) {
          programmaRiconnessione();
        }
      }),
    );

    await connetti();

    const ro = new ResizeObserver(() => {
      try {
        fit.fit();
      } catch {}
    });
    ro.observe(elemento);
    off.push(() => ro.disconnect());
  });

  // Primo tentativo di connessione, con gestione della chiave del server.
  async function connetti(modo) {
    try {
      await avvia(modo);
      dopoConnesso();
    } catch (e) {
      const msg = String(e);
      // Errore speciale "HOSTKEY:<stato>:<impronta>": chiediamo all'utente.
      if (msg.includes("HOSTKEY:") && onHostKey) {
        const dopo = msg.slice(msg.indexOf("HOSTKEY:") + 8);
        const [stato, ...resto] = dopo.split(":");
        onHostKey({ id: tab.id, stato, impronta: resto.join(":") });
        return;
      }
      term.write(`\r\n\x1b[31mErrore: ${msg}\x1b[0m\r\n`);
      onErrore?.(String(e));
    }
  }

  // Riprova a connettersi con attesa crescente (1s, 2s, 4s… max 30s).
  function programmaRiconnessione() {
    if (distrutto) return;
    tentativi++;
    const attesa = Math.min(30000, 1000 * 2 ** (tentativi - 1));
    term.write(`\x1b[33m[riconnessione tra ${attesa / 1000}s…]\x1b[0m\r\n`);
    timerRiconn = setTimeout(async () => {
      try {
        await avvia();
        onConnesso?.();
        tentativi = 0;
        term.write("\x1b[32m[riconnesso]\x1b[0m\r\n");
        term.focus();
      } catch {
        programmaRiconnessione();
      }
    }, attesa);
  }

  // Operazioni da fare una volta connessi (collega input, resize, scorciatoie).
  function dopoConnesso() {
    tentativi = 0;
    onConnesso?.();
    term.onData((d) => invia(tab.id, d));
    term.onResize(({ cols, rows }) => api.termRidimensiona(tab.id, cols, rows));
    term.attachCustomKeyEventHandler((ev) => {
      if (ev.ctrlKey && ev.shiftKey && ev.key === "F" && ev.type === "keydown") {
        mostraCerca = !mostraCerca;
        return false;
      }
      return true;
    });
    term.focus();
    // Comandi automatici all'avvio (dopo un attimo, per far comparire il prompt).
    if (tab.comandi_avvio) {
      setTimeout(() => api.termScrivi(tab.id, tab.comandi_avvio + "\n"), 400);
    }
  }

  // Ritenta la connessione fidandosi della chiave (chiamata da App dopo conferma).
  export async function riprovaConFiducia(modo) {
    await connetti(modo);
  }

  // Avvia la connessione adatta al tipo di sessione.
  async function avvia(modo) {
    if (tab.tipo === "ssh") {
      await api.sshConnetti({
        id: tab.id,
        host: tab.host,
        porta: tab.porta,
        utente: tab.utente,
        auth: tab.auth,
        colonne: term.cols,
        righe: term.rows,
        modo,
        jump: tab.jump,
      });
    } else if (tab.tipo === "locale") {
      await api.apriLocale(tab.id, tab.shell, term.cols, term.rows);
    } else if (tab.tipo === "telnet") {
      await api.apriTelnet(tab.id, tab.host, tab.porta);
    } else if (tab.tipo === "seriale") {
      await api.apriSeriale(tab.id, tab.porta_seriale, tab.baud);
    }
  }

  function cerca(avanti = true) {
    if (!testoCerca) return;
    if (avanti) search.findNext(testoCerca);
    else search.findPrevious(testoCerca);
  }

  // ---- Funzioni esposte ad App (via bind:this) ----

  // Pulisce lo schermo del terminale.
  export function pulisci() {
    term?.clear();
  }

  // Cambia la dimensione del carattere (zoom).
  export function zoom(delta) {
    impostazioni.fontSize = Math.max(8, Math.min(32, impostazioni.fontSize + delta));
  }

  // Riavvia la connessione (utile dopo una disconnessione).
  export async function riconnetti() {
    if (!term) return;
    term.write("\r\n\x1b[33m[riconnessione…]\x1b[0m\r\n");
    try {
      await avvia();
      onConnesso?.();
      term.focus();
    } catch (e) {
      term.write(`\r\n\x1b[31mErrore: ${e}\x1b[0m\r\n`);
      onErrore?.(String(e));
    }
  }

  // Riapplica le impostazioni a caldo quando cambiano.
  $effect(() => {
    if (!term) return;
    term.options.fontFamily = impostazioni.fontFamily;
    term.options.fontSize = impostazioni.fontSize;
    term.options.cursorBlink = impostazioni.cursorBlink;
    term.options.scrollback = impostazioni.scrollback;
    term.options.theme = temi[impostazioni.tema] ?? temi.Scuro;
    try {
      fit?.fit();
    } catch {}
  });

  // Quando il tab torna attivo, rifai il fit e dai il focus.
  $effect(() => {
    if (attivo && fit) {
      setTimeout(() => {
        try {
          fit.fit();
        } catch {}
        term?.focus();
      }, 0);
    }
  });

  onDestroy(() => {
    distrutto = true;
    clearTimeout(timerRiconn);
    off.forEach((f) => f());
    term?.dispose();
  });
</script>

<div class="term-area">
  {#if mostraCerca}
    <div class="cerca">
      <input
        placeholder="Cerca…"
        bind:value={testoCerca}
        onkeydown={(e) => {
          if (e.key === "Enter") cerca(!e.shiftKey);
          if (e.key === "Escape") (mostraCerca = false);
        }}
      />
      <button onclick={() => cerca(false)} title="Precedente">↑</button>
      <button onclick={() => cerca(true)} title="Successivo">↓</button>
      <button onclick={() => (mostraCerca = false)}>✕</button>
    </div>
  {/if}
  <div class="term" bind:this={elemento}></div>
</div>
