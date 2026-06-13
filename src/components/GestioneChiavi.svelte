<script>
  // Gestione delle chiavi SSH locali: elenco, generazione e copia sul server
  // (ssh-copy-id) usando la sessione SSH attiva.
  import { onMount } from "svelte";
  import * as api from "../lib/api.js";

  // `idSessione` = id della sessione SSH attiva (per "copia sul server"), o null.
  let { idSessione = null, onChiudi } = $props();

  let chiavi = $state([]);
  let nome = $state("id_oxiterm");
  let commento = $state("oxiterm");
  let messaggio = $state("");

  onMount(carica);

  async function carica() {
    try {
      chiavi = await api.listaChiavi();
    } catch (e) {
      messaggio = String(e);
    }
  }

  async function genera() {
    messaggio = "Generazione…";
    try {
      await api.generaChiave(nome, commento);
      messaggio = "✓ Chiave creata in ~/.ssh/" + nome;
      carica();
    } catch (e) {
      messaggio = String(e);
    }
  }

  async function copia(k) {
    if (!idSessione) {
      messaggio = "Apri prima una sessione SSH per copiare la chiave.";
      return;
    }
    messaggio = "Copia sul server…";
    try {
      await api.copiaChiave(idSessione, k.contenuto);
      messaggio = "✓ Chiave copiata in authorized_keys";
    } catch (e) {
      messaggio = String(e);
    }
  }
</script>

<div class="overlay" onclick={onChiudi}>
  <div class="modale" onclick={(e) => e.stopPropagation()} style="width:560px;max-width:92vw">
    <h2>Chiavi SSH</h2>

    <div class="riga">
      <div class="campo" style="flex:2">
        <label>Nome file</label>
        <input bind:value={nome} />
      </div>
      <div class="campo" style="flex:2">
        <label>Commento</label>
        <input bind:value={commento} />
      </div>
    </div>
    <button class="primario" onclick={genera}>+ Genera chiave Ed25519</button>

    {#if messaggio}<p style="font-size:12px;color:var(--testo2)">{messaggio}</p>{/if}

    <h2 style="font-size:14px;margin-top:18px">Chiavi in ~/.ssh</h2>
    <div style="max-height:280px;overflow:auto">
      {#each chiavi as k (k.percorso)}
        <div class="riga-file" style="flex-direction:column;align-items:stretch;gap:4px">
          <div style="display:flex;justify-content:space-between;gap:8px">
            <b>{k.nome}</b>
            <button
              disabled={!idSessione}
              title={idSessione ? "Copia sul server attivo" : "Nessuna sessione SSH attiva"}
              onclick={() => copia(k)}>Copia sul server ▶</button
            >
          </div>
          <code style="font-size:10px;color:var(--testo2);word-break:break-all">{k.contenuto}</code>
        </div>
      {:else}
        <div class="vuoto">Nessuna chiave pubblica trovata.</div>
      {/each}
    </div>

    <div class="pulsanti">
      <button onclick={onChiudi}>Chiudi</button>
    </div>
  </div>
</div>
