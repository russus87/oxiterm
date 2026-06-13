<script>
  // Gestione dei tunnel SSH (port forwarding) di una sessione: elenco, creazione
  // di un forward locale (-L) o di un proxy SOCKS5 dinamico (-D), e stop.
  import { onMount } from "svelte";
  import * as api from "../lib/api.js";

  let { id, onChiudi } = $props();

  let tunnel = $state([]);
  let errore = $state("");
  let tipo = $state("locale"); // locale | socks | remoto
  let portaLocale = $state(8080);
  let hostRemoto = $state("");
  let portaRemota = $state(80);
  let hostLocale = $state("localhost");

  onMount(aggiorna);

  async function aggiorna() {
    try {
      tunnel = await api.listaTunnel(id);
    } catch (e) {
      errore = String(e);
    }
  }

  async function crea() {
    errore = "";
    try {
      if (tipo === "locale") {
        await api.tunnelLocale(id, Number(portaLocale), hostRemoto, Number(portaRemota));
      } else if (tipo === "remoto") {
        await api.tunnelRemoto(id, Number(portaRemota), hostLocale, Number(portaLocale));
      } else {
        await api.tunnelSocks(id, Number(portaLocale));
      }
      aggiorna();
    } catch (e) {
      errore = String(e);
    }
  }

  async function ferma(t) {
    await api.fermaTunnel(id, t.id).catch((e) => (errore = String(e)));
    aggiorna();
  }
</script>

<div class="overlay" onclick={onChiudi}>
  <div class="modale" onclick={(e) => e.stopPropagation()} style="width:440px">
    <h2>Tunnel SSH</h2>

    <div class="campo">
      <label>Tipo</label>
      <select bind:value={tipo}>
        <option value="locale">Forward locale (-L)</option>
        <option value="remoto">Forward remoto (-R)</option>
        <option value="socks">Proxy dinamico SOCKS5 (-D)</option>
      </select>
    </div>

    {#if tipo === "locale"}
      <div class="riga">
        <div class="campo" style="flex:1">
          <label>Porta locale</label>
          <input type="number" bind:value={portaLocale} />
        </div>
        <div class="campo" style="flex:2">
          <label>Host remoto</label>
          <input bind:value={hostRemoto} placeholder="localhost o IP interno" />
        </div>
        <div class="campo" style="flex:1">
          <label>Porta remota</label>
          <input type="number" bind:value={portaRemota} />
        </div>
      </div>
      <p style="color:var(--testo2);font-size:11px;margin:0 0 10px">
        La porta locale verrà inoltrata, via SSH, a host:porta remoti.
      </p>
    {:else if tipo === "remoto"}
      <div class="riga">
        <div class="campo" style="flex:1">
          <label>Porta remota</label>
          <input type="number" bind:value={portaRemota} />
        </div>
        <div class="campo" style="flex:2">
          <label>Host locale</label>
          <input bind:value={hostLocale} placeholder="localhost o IP raggiungibile da te" />
        </div>
        <div class="campo" style="flex:1">
          <label>Porta locale</label>
          <input type="number" bind:value={portaLocale} />
        </div>
      </div>
      <p style="color:var(--testo2);font-size:11px;margin:0 0 10px">
        Il server ascolterà sulla porta remota e inoltrerà a host:porta dal tuo lato.
      </p>
    {:else}
      <div class="campo">
        <label>Porta locale del proxy</label>
        <input type="number" bind:value={portaLocale} />
      </div>
      <p style="color:var(--testo2);font-size:11px;margin:0 0 10px">
        Imposta il browser su SOCKS5 127.0.0.1:{portaLocale} per navigare tramite il server.
      </p>
    {/if}

    <button class="primario" onclick={crea}>+ Crea tunnel</button>

    {#if errore}<p style="color:#ff8787">{errore}</p>{/if}

    <h2 style="font-size:14px;margin-top:18px">Tunnel attivi</h2>
    {#each tunnel as t (t.id)}
      <div class="riga-file">
        <span class="nome">{t.descrizione}</span>
        <button class="pericolo" onclick={() => ferma(t)}>Ferma</button>
      </div>
    {:else}
      <div class="vuoto">Nessun tunnel attivo.</div>
    {/each}

    <div class="pulsanti">
      <button onclick={onChiudi}>Chiudi</button>
    </div>
  </div>
</div>
