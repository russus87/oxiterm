<script>
  // Pannello impostazioni: aspetto del terminale e comportamento. I valori sono
  // legati direttamente allo store reattivo `impostazioni`, che si salva da solo.
  import { onMount } from "svelte";
  import { impostazioni } from "../lib/impostazioni.svelte.js";
  import { nomiTemi } from "../lib/temi.js";
  import * as api from "../lib/api.js";

  let { onChiudi } = $props();

  let remote = $state("");
  let messaggio = $state("");

  onMount(async () => {
    try {
      remote = (await api.syncRemote()) || "";
    } catch {}
  });

  async function impostaRemote() {
    try {
      await api.syncImpostaRemote(remote);
      messaggio = "Remote impostato.";
    } catch (e) {
      messaggio = String(e);
    }
  }
  async function push() {
    messaggio = "Invio in corso…";
    try {
      await api.syncPush();
      messaggio = "✓ Rubrica inviata.";
    } catch (e) {
      messaggio = String(e);
    }
  }
  async function pull() {
    messaggio = "Scaricamento in corso…";
    try {
      await api.syncPull();
      messaggio = "✓ Rubrica aggiornata (riavvia per ricaricarla).";
    } catch (e) {
      messaggio = String(e);
    }
  }
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

    <div class="campo" style="display:flex;align-items:center;gap:8px">
      <input type="checkbox" bind:checked={impostazioni.autoReconnect} style="width:auto" id="recon" />
      <label for="recon" style="margin:0">Riconnetti automaticamente le sessioni cadute</label>
    </div>

    <h2 style="font-size:14px;margin-top:18px">Sincronizzazione cloud (Git)</h2>
    <p style="color:var(--testo2);font-size:11px;margin:0 0 8px">
      Salva la rubrica in un repository Git (GitHub/GitLab…) per averla su più PC.
    </p>
    <div class="campo">
      <label>URL del repository</label>
      <div class="riga">
        <input bind:value={remote} placeholder="git@github.com:utente/oxiterm-config.git" />
        <button onclick={impostaRemote}>Imposta</button>
      </div>
    </div>
    <div class="riga">
      <button style="flex:1" onclick={push}>⬆ Push</button>
      <button style="flex:1" onclick={pull}>⬇ Pull</button>
    </div>
    {#if messaggio}<p style="font-size:12px;color:var(--testo2)">{messaggio}</p>{/if}

    <div class="pulsanti">
      <button class="primario" onclick={onChiudi}>Chiudi</button>
    </div>
  </div>
</div>
