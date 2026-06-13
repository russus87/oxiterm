<script>
  // Editor di testo integrato per file remoti: carica il contenuto via SFTP,
  // permette di modificarlo e salvarlo di nuovo sul server.
  import { onMount } from "svelte";
  import * as api from "../lib/api.js";

  let { id, percorso, onChiudi } = $props();

  let contenuto = $state("");
  let stato = $state("Caricamento…");

  onMount(async () => {
    try {
      contenuto = await api.sftpLeggiTesto(id, percorso);
      stato = "";
    } catch (e) {
      stato = "Errore: " + e;
    }
  });

  async function salva() {
    stato = "Salvataggio…";
    try {
      await api.sftpScriviTesto(id, percorso, contenuto);
      stato = "✓ Salvato";
    } catch (e) {
      stato = "Errore: " + e;
    }
  }
</script>

<div class="overlay" onclick={onChiudi}>
  <div class="modale" onclick={(e) => e.stopPropagation()} style="width:720px;max-width:92vw">
    <h2 style="font-size:14px">✎ {percorso}</h2>
    <textarea
      bind:value={contenuto}
      spellcheck="false"
      style="width:100%;height:50vh;font-family:monospace;font-size:13px;resize:vertical"
    ></textarea>
    <div class="pulsanti">
      <span style="margin-right:auto;color:var(--testo2);font-size:12px">{stato}</span>
      <button onclick={onChiudi}>Chiudi</button>
      <button class="primario" onclick={salva}>Salva sul server</button>
    </div>
  </div>
</div>
