<script>
  // Libreria di snippet/macro: comandi salvati da inviare al terminale attivo.
  import { onMount } from "svelte";
  import * as api from "../lib/api.js";

  // `onInvia(comando)` invia il comando al terminale attivo. `attivoPresente`
  // dice se c'è una scheda aperta a cui inviare.
  let { onInvia, attivoPresente, onChiudi } = $props();

  let snippet = $state([]);
  let nome = $state("");
  let comando = $state("");
  let errore = $state("");

  onMount(carica);

  async function carica() {
    try {
      snippet = await api.listaSnippet();
    } catch (e) {
      errore = String(e);
    }
  }

  async function aggiungi() {
    if (!nome || !comando) return;
    await api.salvaSnippet({ id: crypto.randomUUID(), nome, comando });
    nome = "";
    comando = "";
    carica();
  }

  async function elimina(s) {
    await api.eliminaSnippet(s.id);
    carica();
  }
</script>

<div class="overlay" onclick={onChiudi}>
  <div class="modale" onclick={(e) => e.stopPropagation()} style="width:480px">
    <h2>Snippet / macro</h2>

    <div class="riga">
      <div class="campo" style="flex:1">
        <label>Nome</label>
        <input bind:value={nome} placeholder="Aggiorna sistema" />
      </div>
      <div class="campo" style="flex:2">
        <label>Comando</label>
        <input bind:value={comando} placeholder="sudo apt update && sudo apt upgrade -y" />
      </div>
    </div>
    <button class="primario" onclick={aggiungi} disabled={!nome || !comando}>+ Aggiungi</button>

    {#if errore}<p style="color:#ff8787">{errore}</p>{/if}

    <h2 style="font-size:14px;margin-top:18px">I miei snippet</h2>
    {#each snippet as s (s.id)}
      <div class="riga-file">
        <span class="nome" title={s.comando}>
          <b>{s.nome}</b> <span class="dett">{s.comando}</span>
        </span>
        <button
          class="primario"
          disabled={!attivoPresente}
          title={attivoPresente ? "Invia al terminale attivo" : "Nessuna scheda aperta"}
          onclick={() => onInvia(s.comando)}>Invia ▶</button
        >
        <button class="pericolo" onclick={() => elimina(s)}>✕</button>
      </div>
    {:else}
      <div class="vuoto">Nessuno snippet. Aggiungine uno qui sopra.</div>
    {/each}

    <div class="pulsanti">
      <button onclick={onChiudi}>Chiudi</button>
    </div>
  </div>
</div>
