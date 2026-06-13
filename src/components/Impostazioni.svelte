<script>
  // Pannello impostazioni: aspetto del terminale e comportamento. I valori sono
  // legati direttamente allo store reattivo `impostazioni`, che si salva da solo.
  import { impostazioni } from "../lib/impostazioni.svelte.js";
  import { nomiTemi } from "../lib/temi.js";

  let { onChiudi } = $props();
</script>

<div class="overlay" onclick={onChiudi}>
  <div class="modale" onclick={(e) => e.stopPropagation()}>
    <h2>Impostazioni</h2>

    <div class="campo">
      <label>Tema colori</label>
      <select bind:value={impostazioni.tema}>
        {#each nomiTemi as t}<option value={t}>{t}</option>{/each}
      </select>
    </div>

    <div class="riga">
      <div class="campo" style="flex:1">
        <label>Dimensione carattere</label>
        <input type="number" min="8" max="32" bind:value={impostazioni.fontSize} />
      </div>
      <div class="campo" style="flex:1">
        <label>Scrollback (righe)</label>
        <input type="number" min="100" step="500" bind:value={impostazioni.scrollback} />
      </div>
    </div>

    <div class="campo">
      <label>Famiglia carattere</label>
      <input bind:value={impostazioni.fontFamily} />
    </div>

    <div class="campo" style="display:flex;align-items:center;gap:8px">
      <input type="checkbox" bind:checked={impostazioni.cursorBlink} style="width:auto" id="blink" />
      <label for="blink" style="margin:0">Cursore lampeggiante</label>
    </div>

    <div class="campo" style="display:flex;align-items:center;gap:8px">
      <input type="checkbox" bind:checked={impostazioni.broadcast} style="width:auto" id="bcast" />
      <label for="bcast" style="margin:0">
        Broadcast: invia l'input a <b>tutte</b> le schede
      </label>
    </div>

    <div class="pulsanti">
      <button class="primario" onclick={onChiudi}>Chiudi</button>
    </div>
  </div>
</div>
