<script>
  // Terminale di una singola sessione SSH.
  // Si occupa di tutto il ciclo di vita: crea xterm, avvia la connessione SSH
  // (con la dimensione reale del terminale), inoltra l'input e mostra l'output.
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { listen } from "@tauri-apps/api/event";
  import * as api from "../lib/api.js";

  // Proprietà: dati di connessione + callback verso App.
  let { tab, attivo, onConnesso, onErrore, onChiuso } = $props();

  let elemento;
  let term, fit;
  let off = []; // funzioni per rimuovere i listener

  onMount(async () => {
    term = new Terminal({
      fontFamily: "Consolas, 'DejaVu Sans Mono', monospace",
      fontSize: 14,
      cursorBlink: true,
      theme: { background: "#1e1e1e", foreground: "#d4d4d4" },
    });
    fit = new FitAddon();
    term.loadAddon(fit);
    term.open(elemento);
    fit.fit();

    // Ascolta l'output del server e gli eventi di chiusura per QUESTA sessione.
    off.push(
      await listen(`ssh-dati-${tab.id}`, (e) =>
        term.write(new Uint8Array(e.payload)),
      ),
    );
    off.push(
      await listen(`ssh-chiuso-${tab.id}`, () => {
        term.write("\r\n\x1b[31m[connessione chiusa]\x1b[0m\r\n");
        onChiuso?.();
      }),
    );

    // Avvia la connessione con la dimensione attuale del terminale.
    try {
      await api.sshConnetti({
        id: tab.id,
        host: tab.host,
        porta: tab.porta,
        utente: tab.utente,
        auth: tab.auth,
        colonne: term.cols,
        righe: term.rows,
      });
      onConnesso?.();
      // Inoltra al server tutto ciò che l'utente digita.
      term.onData((d) => api.sshScrivi(tab.id, d));
      term.onResize(({ cols, rows }) => api.sshRidimensiona(tab.id, cols, rows));
      term.focus();
    } catch (e) {
      term.write(`\r\n\x1b[31mErrore: ${e}\x1b[0m\r\n`);
      onErrore?.(String(e));
    }

    // Ridimensiona il terminale quando cambia la dimensione del contenitore.
    const ro = new ResizeObserver(() => {
      try {
        fit.fit();
      } catch {}
    });
    ro.observe(elemento);
    off.push(() => ro.disconnect());
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

<div class="term" bind:this={elemento}></div>
