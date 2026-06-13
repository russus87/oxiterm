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
  import * as api from "../lib/api.js";
  import { impostazioni } from "../lib/impostazioni.svelte.js";
  import { temi } from "../lib/temi.js";

  // Proprietà: dati del tab + callback verso App.
  // `invia` decide se mandare l'input solo a questa scheda o a tutte (broadcast).
  let { tab, attivo, invia, onConnesso, onErrore, onChiuso } = $props();

  let elemento;
  let term, fit, search;
  let off = [];
  let mostraCerca = $state(false);
  let testoCerca = $state("");

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
    term.loadAddon(new WebLinksAddon());
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
      }),
    );

    try {
      await avvia();
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
    } catch (e) {
      term.write(`\r\n\x1b[31mErrore: ${e}\x1b[0m\r\n`);
      onErrore?.(String(e));
    }

    const ro = new ResizeObserver(() => {
      try {
        fit.fit();
      } catch {}
    });
    ro.observe(elemento);
    off.push(() => ro.disconnect());
  });

  // Avvia la connessione adatta al tipo di sessione.
  async function avvia() {
    if (tab.tipo === "ssh") {
      await api.sshConnetti({
        id: tab.id,
        host: tab.host,
        porta: tab.porta,
        utente: tab.utente,
        auth: tab.auth,
        colonne: term.cols,
        righe: term.rows,
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
