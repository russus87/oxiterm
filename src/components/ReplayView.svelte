<script>
  // Replay di una sessione registrata (formato asciicast v2): la riproduce in un
  // terminale xterm rispettando i tempi originali.
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { open as apriFile } from "@tauri-apps/plugin-dialog";
  import * as api from "../lib/api.js";
  import { impostazioni } from "../lib/impostazioni.svelte.js";
  import { temi } from "../lib/temi.js";

  let { onChiudi } = $props();

  let elemento;
  let term, fit;
  let timers = [];
  let stato = $state("Scegli un file .cast da riprodurre.");

  onMount(() => {
    term = new Terminal({
      fontFamily: impostazioni.fontFamily,
      fontSize: impostazioni.fontSize,
      theme: temi[impostazioni.tema] ?? temi.Scuro,
    });
    fit = new FitAddon();
    term.loadAddon(fit);
    term.open(elemento);
    setTimeout(() => fit.fit(), 0);
  });

  function ferma() {
    timers.forEach(clearTimeout);
    timers = [];
  }

  async function scegli() {
    const f = await apriFile({ multiple: false, filters: [{ name: "asciicast", extensions: ["cast"] }] });
    if (!f) return;
    try {
      const testo = await api.leggiFile(f);
      riproduci(testo);
    } catch (e) {
      stato = String(e);
    }
  }

  function riproduci(testo) {
    ferma();
    term.reset();
    stato = "Riproduzione…";
    const righe = testo.split("\n").filter(Boolean);
    // La prima riga è l'intestazione JSON, le altre sono eventi [t, "o", dati].
    let ultimo = 0;
    for (let i = 1; i < righe.length; i++) {
      try {
        const ev = JSON.parse(righe[i]);
        if (Array.isArray(ev) && ev[1] === "o") {
          ultimo = ev[0];
          timers.push(setTimeout(() => term.write(ev[2]), ev[0] * 1000));
        }
      } catch {}
    }
    timers.push(setTimeout(() => (stato = "✓ Fine"), ultimo * 1000 + 50));
  }

  onDestroy(() => {
    ferma();
    term?.dispose();
  });
</script>

<div class="overlay" onclick={onChiudi}>
  <div class="modale" onclick={(e) => e.stopPropagation()} style="width:820px;max-width:94vw">
    <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:8px">
      <h2 style="margin:0">Replay sessione</h2>
      <div style="display:flex;gap:8px;align-items:center">
        <span style="color:var(--testo2);font-size:12px">{stato}</span>
        <button onclick={scegli}>Apri .cast</button>
        <button onclick={ferma}>Ferma</button>
        <button onclick={onChiudi}>Chiudi</button>
      </div>
    </div>
    <div bind:this={elemento} style="height:55vh;background:#1e1e1e"></div>
  </div>
</div>
