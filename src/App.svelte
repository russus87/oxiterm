<script>
  // Componente principale: sidebar col session manager (a gruppi) + area a schede
  // con terminale e, per le sessioni SSH, pannello SFTP. Gestisce impostazioni,
  // tunnel e broadcast dell'input.
  import { onMount } from "svelte";
  import { open as apriFile, save as salvaFile } from "@tauri-apps/plugin-dialog";
  import * as api from "./lib/api.js";
  import { impostazioni } from "./lib/impostazioni.svelte.js";
  import Terminale from "./components/Terminale.svelte";
  import Sftp from "./components/Sftp.svelte";
  import FormConnessione from "./components/FormConnessione.svelte";
  import Impostazioni from "./components/Impostazioni.svelte";
  import Tunnel from "./components/Tunnel.svelte";
  import Snippet from "./components/Snippet.svelte";
  import Info from "./components/Info.svelte";

  let sessioni = $state([]); // rubrica salvata
  let tabs = $state([]); // sessioni aperte
  let tabAttivoId = $state(null);

  let mostraForm = $state(false);
  let formIniziale = $state(null);
  let mostraImpostazioni = $state(false);
  let mostraTunnel = $state(false);
  let mostraSnippet = $state(false);
  let mostraInfo = $state(false);

  // Riferimenti ai componenti Terminale, per chiamarne le funzioni (pulisci, zoom…).
  let refsTerm = {};

  const tabAttivo = $derived(tabs.find((t) => t.id === tabAttivoId) ?? null);

  // Raggruppa le sessioni salvate per "gruppo".
  const gruppi = $derived.by(() => {
    const m = new Map();
    for (const s of sessioni) {
      const g = s.gruppo || "Senza gruppo";
      if (!m.has(g)) m.set(g, []);
      m.get(g).push(s);
    }
    return [...m.entries()];
  });

  onMount(caricaSessioni);

  async function caricaSessioni() {
    try {
      sessioni = await api.listaSessioni();
    } catch (e) {
      console.error(e);
    }
  }

  function nuovaConnessione() {
    formIniziale = null;
    mostraForm = true;
  }

  function apriSalvata(s) {
    formIniziale = {
      idSalvata: s.id,
      tipo: s.tipo || "ssh",
      nome: s.nome,
      host: s.host || "",
      porta: s.porta || 22,
      utente: s.utente || "",
      metodo: s.chiave ? "chiave" : "password",
      percorsoChiave: s.chiave || "",
      porta_seriale: s.porta_seriale || "",
      baud: s.baud || 115200,
      gruppo: s.gruppo || "",
      colore: s.colore || "#37b24d",
      salva: false,
    };
    mostraForm = true;
  }

  async function eliminaSalvata(s) {
    if (!confirm(`Eliminare la sessione salvata "${s.nome}"?`)) return;
    await api.eliminaSessione(s.id);
    caricaSessioni();
  }

  // Apre una nuova scheda a partire dal form di connessione.
  async function connetti(form) {
    const auth =
      form.metodo === "password"
        ? { tipo: "password", password: form.password }
        : {
            tipo: "chiave",
            percorso: form.percorsoChiave,
            passphrase: form.passphrase || null,
          };

    const nome =
      form.nome ||
      (form.tipo === "ssh"
        ? `${form.utente}@${form.host}`
        : form.tipo === "telnet"
          ? `telnet ${form.host}`
          : form.tipo === "seriale"
            ? form.porta_seriale
            : "locale");

    const tab = {
      id: crypto.randomUUID(),
      tipo: form.tipo,
      nome,
      host: form.host,
      porta: Number(form.porta),
      utente: form.utente,
      auth,
      shell: form.shell,
      porta_seriale: form.porta_seriale,
      baud: Number(form.baud),
      connesso: false,
    };

    if (form.salva) {
      await api.salvaSessione({
        id: form.idSalvata || crypto.randomUUID(),
        nome,
        tipo: form.tipo,
        host: form.host,
        porta: Number(form.porta),
        utente: form.utente,
        chiave: form.metodo === "chiave" ? form.percorsoChiave : null,
        gruppo: form.gruppo || null,
        colore: form.colore || null,
        porta_seriale: form.porta_seriale || null,
        baud: form.tipo === "seriale" ? Number(form.baud) : null,
      });
      caricaSessioni();
    }

    tabs.push(tab);
    tabAttivoId = tab.id;
    mostraForm = false;
  }

  async function chiudiTab(id) {
    await api.termChiudi(id).catch(() => {});
    delete refsTerm[id];
    const i = tabs.findIndex((t) => t.id === id);
    tabs = tabs.filter((t) => t.id !== id);
    if (tabAttivoId === id) {
      tabAttivoId = tabs[Math.max(0, i - 1)]?.id ?? null;
    }
  }

  // Inoltra l'input: a una sola scheda o a tutte (broadcast).
  function invia(id, dati) {
    if (impostazioni.broadcast) {
      for (const t of tabs) if (t.connesso) api.termScrivi(t.id, dati);
    } else {
      api.termScrivi(id, dati);
    }
  }

  // Invia un comando (snippet) al terminale attivo, con a capo finale.
  function inviaSnippet(comando) {
    if (tabAttivo) api.termScrivi(tabAttivo.id, comando + "\n");
  }

  // Apre una nuova scheda con gli stessi parametri di una esistente.
  function duplica(t) {
    const copia = { ...t, id: crypto.randomUUID(), connesso: false };
    tabs.push(copia);
    tabAttivoId = copia.id;
  }

  // Azioni sul terminale attivo (delegano al componente via ref).
  const azione = (fn, ...args) => {
    const r = refsTerm[tabAttivoId];
    if (r && r[fn]) r[fn](...args);
  };

  // Esporta la rubrica su file.
  async function esporta() {
    const dest = await salvaFile({ defaultPath: "oxiterm-sessioni.json" });
    if (dest) await api.esportaRubrica(dest).catch((e) => alert(e));
  }

  // Importa la rubrica da file e ricarica.
  async function importa() {
    const f = await apriFile({ multiple: false });
    if (!f) return;
    try {
      const n = await api.importaRubrica(f);
      await caricaSessioni();
      alert(`Importate ${n} nuove sessioni.`);
    } catch (e) {
      alert(e);
    }
  }

  function icona(tipo) {
    return { ssh: "🔐", locale: "💻", telnet: "🌐", seriale: "🔌" }[tipo] || "🖥";
  }
</script>

<div class="app">
  <!-- Sidebar: session manager a gruppi -->
  <aside class="sidebar">
    <h1><span class="pallino"></span> Oxiterm</h1>
    <div class="azioni">
      <button class="primario" onclick={nuovaConnessione}>+ Nuova sessione</button>
      <div style="display:flex;gap:6px;margin-top:6px">
        <button style="flex:1" title="Importa rubrica" onclick={importa}>⬇ Importa</button>
        <button style="flex:1" title="Esporta rubrica" onclick={esporta}>⬆ Esporta</button>
      </div>
    </div>
    <div class="lista-sessioni">
      {#each gruppi as [nomeGruppo, lista] (nomeGruppo)}
        <div class="gruppo">{nomeGruppo}</div>
        {#each lista as s (s.id)}
          <div class="voce-sessione" onclick={() => apriSalvata(s)}>
            <span class="nome" style={s.colore ? `color:${s.colore}` : ""}>
              {icona(s.tipo)} {s.nome}
            </span>
            <span class="dett">
              {#if s.tipo === "ssh"}{s.utente}@{s.host}:{s.porta}
              {:else if s.tipo === "telnet"}telnet {s.host}:{s.porta}
              {:else if s.tipo === "seriale"}{s.porta_seriale} @ {s.baud}
              {:else}terminale locale{/if}
            </span>
            <button
              class="x pericolo"
              title="Elimina"
              onclick={(e) => (e.stopPropagation(), eliminaSalvata(s))}>elimina</button
            >
          </div>
        {/each}
      {:else}
        <div class="vuoto">Nessuna sessione salvata.<br />Creane una con "+ Nuova sessione".</div>
      {/each}
    </div>
    <div class="azioni" style="display:flex;gap:6px">
      <button style="flex:1" onclick={() => (mostraSnippet = true)}>✂ Snippet</button>
      <button style="flex:1" onclick={() => (mostraImpostazioni = true)}>⚙ Opzioni</button>
      <button title="Info" onclick={() => (mostraInfo = true)}>ℹ</button>
    </div>
  </aside>

  <!-- Area principale -->
  <main class="principale">
    <div class="tabbar">
      {#each tabs as t (t.id)}
        <div
          class="tab {t.id === tabAttivoId ? 'attivo' : ''}"
          onclick={() => (tabAttivoId = t.id)}
        >
          <span>{icona(t.tipo)} {t.nome}</span>
          <button class="x" onclick={(e) => (e.stopPropagation(), chiudiTab(t.id))}>✕</button>
        </div>
      {/each}
      <div style="flex:1"></div>
      {#if impostazioni.broadcast}
        <div class="tab" style="color:#ffd43b" title="Broadcast attivo">📢 broadcast</div>
      {/if}
      {#if tabAttivo}
        {#if !tabAttivo.connesso}
          <button class="strumento" title="Riconnetti" onclick={() => azione("riconnetti")}>↻ Riconnetti</button>
        {/if}
        <button class="strumento" title="Pulisci schermo" onclick={() => azione("pulisci")}>🧹</button>
        <button class="strumento" title="Riduci testo" onclick={() => azione("zoom", -1)}>A−</button>
        <button class="strumento" title="Ingrandisci testo" onclick={() => azione("zoom", 1)}>A+</button>
        <button class="strumento" title="Duplica scheda" onclick={() => duplica(tabAttivo)}>⧉</button>
        {#if tabAttivo.tipo === "ssh"}
          <button class="strumento" title="Tunnel SSH" onclick={() => (mostraTunnel = true)}>🚇 Tunnel</button>
        {/if}
      {/if}
    </div>

    <div class="contenuto">
      {#if tabs.length === 0}
        <div class="benvenuto">
          <div style="font-size:40px">🦀</div>
          <div>Nessuna sessione aperta.</div>
          <button class="primario" onclick={nuovaConnessione}>+ Nuova sessione</button>
        </div>
      {/if}
      {#each tabs as t (t.id)}
        <div
          class="pannello-tab {t.id === tabAttivoId ? '' : 'nascosto'} {t.tipo === 'ssh'
            ? ''
            : 'solo-term'}"
        >
          <div class="term-wrap">
            <Terminale
              bind:this={refsTerm[t.id]}
              tab={t}
              attivo={t.id === tabAttivoId}
              {invia}
              onConnesso={() => (t.connesso = true)}
              onChiuso={() => (t.connesso = false)}
            />
          </div>
          {#if t.tipo === "ssh"}
            <Sftp id={t.id} pronto={t.connesso} />
          {/if}
        </div>
      {/each}
    </div>
  </main>
</div>

{#if mostraForm}
  <FormConnessione
    iniziale={formIniziale}
    onConnetti={connetti}
    onChiudi={() => (mostraForm = false)}
  />
{/if}

{#if mostraImpostazioni}
  <Impostazioni onChiudi={() => (mostraImpostazioni = false)} />
{/if}

{#if mostraTunnel && tabAttivo}
  <Tunnel id={tabAttivo.id} onChiudi={() => (mostraTunnel = false)} />
{/if}

{#if mostraSnippet}
  <Snippet
    onInvia={inviaSnippet}
    attivoPresente={!!tabAttivo}
    onChiudi={() => (mostraSnippet = false)}
  />
{/if}

{#if mostraInfo}
  <Info onChiudi={() => (mostraInfo = false)} />
{/if}
