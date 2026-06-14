<script>
  // Monitor del server: snapshot periodico (uptime/memoria/disco/processi) via SSH.
  import { onMount, onDestroy } from "svelte";
  import * as api from "../lib/api.js";

  let { id, onChiudi } = $props();

  let testo = $state("Caricamento…");
  let timer;

  async function aggiorna() {
    try {
      testo = await api.serverStato(id);
    } catch (e) {
      testo = String(e);
    }
  }

  onMount(() => {
    aggiorna();
    timer = setInterval(aggiorna, 3000);
  });
  onDestroy(() => clearInterval(timer));
</script>

<div class="overlay" onclick={onChiudi}>
  <div class="modale" onclick={(e) => e.stopPropagation()} style="width:720px;max-width:94vw">
    <div style="display:flex;justify-content:space-between;align-items:center">
      <h2 style="margin:0">📊 Monitor server</h2>
      <span style="color:var(--testo2);font-size:11px">aggiornamento ogni 3s</span>
    </div>
    <pre style="margin-top:10px;max-height:60vh;overflow:auto;background:var(--sfondo);padding:10px;border-radius:6px;font-size:12px;white-space:pre-wrap">{testo}</pre>
    <div class="pulsanti">
      <button class="primario" onclick={onChiudi}>Chiudi</button>
    </div>
  </div>
</div>
