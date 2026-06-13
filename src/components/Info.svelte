<script>
  // Pannello Info: versione, breve descrizione e changelog.
  import { VERSIONE, changelog } from "../lib/versione.js";
  let { onChiudi } = $props();
</script>

<div class="overlay" onclick={onChiudi}>
  <div class="modale" onclick={(e) => e.stopPropagation()} style="width:460px">
    <h2>🦀 Oxiterm <span style="color:var(--testo2);font-weight:400">v{VERSIONE}</span></h2>
    <p style="color:var(--testo2);margin-top:0">
      Terminale SSH/SFTP a schede in stile MobaXterm, scritto in Rust + Tauri + Svelte.
    </p>

    <h2 style="font-size:14px">Novità</h2>
    <div style="max-height:300px;overflow-y:auto">
      {#each changelog as c (c.versione)}
        <div style="margin-bottom:12px">
          <b>v{c.versione}</b>
          <ul style="margin:4px 0 0;padding-left:18px;color:var(--testo2)">
            {#each c.note as n}<li>{n}</li>{/each}
          </ul>
        </div>
      {/each}
    </div>

    <div class="pulsanti">
      <button class="primario" onclick={onChiudi}>Chiudi</button>
    </div>
  </div>
</div>
