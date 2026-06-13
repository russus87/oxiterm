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
  import VncView from "./components/VncView.svelte";
  import PaletteComandi from "./components/PaletteComandi.svelte";
  import CodaTrasferimenti from "./components/CodaTrasferimenti.svelte";

  let sessioni = $state([]); // rubrica salvata
  let tabs = $state([]); // sessioni aperte
  let tabAttivoId = $state(null);

  let mostraForm = $state(false);
  let formIniziale = $state(null);
  let mostraImpostazioni = $state(false);
  let mostraTunnel = $state(false);
  let mostraSnippet = $state(false);
  let mostraInfo = $state(false);
  let mostraPalette = $state(false);
  let hostKey = $state(null); // { id, stato, impronta } per il prompt known_hosts

  // Scorciatoie globali da tastiera.
  function scorciatoie(e) {
    if (e.ctrlKey && (e.key === "p" || e.key === "P")) {
      e.preventDefault();
      mostraPalette = !mostraPalette;
    }
  }

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
      usaJump: !!s.jump_host,
      jump_host: s.jump_host || "",
      jump_porta: s.jump_porta || 22,
      jump_utente: s.jump_utente || "",
      jump_metodo: s.jump_chiave ? "chiave" : "password",
      jump_chiave: s.jump_chiave || "",
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

    // Eventuale jump host (ProxyJump).
    const jump = form.usaJump
      ? {
          host: form.jump_host,
          porta: Number(form.jump_porta),
          utente: form.jump_utente,
          auth:
            form.jump_metodo === "password"
              ? { tipo: "password", password: form.jump_password }
              : {
                  tipo: "chiave",
                  percorso: form.jump_chiave,
                  passphrase: form.jump_passphrase || null,
                },
        }
      : null;

    const nome =
      form.nome ||
      (form.tipo === "ssh"
        ? `${form.utente}@${form.host}`
        : form.tipo === "telnet"
          ? `telnet ${form.host}`
          : form.tipo === "vnc"
            ? `vnc ${form.host}`
            : form.tipo === "seriale"
              ? form.porta_seriale
              : "locale");

    const tid = crypto.randomUUID();
    const tab = {
      id: tid,
      tipo: form.tipo,
      nome,
      host: form.host,
      porta: Number(form.porta),
      utente: form.utente,
      auth,
      jump,
      vnc_password: form.vnc_password,
      shell: form.shell,
      porta_seriale: form.porta_seriale,
      baud: Number(form.baud),
      colore: form.colore,
      layout: "singolo", // singolo | h (affiancati) | v (impilati)
      panes: [{ pid: tid, connesso: false }],
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
        jump_host: form.usaJump ? form.jump_host : null,
        jump_porta: form.usaJump ? Number(form.jump_porta) : null,
        jump_utente: form.usaJump ? form.jump_utente : null,
        jump_chiave:
          form.usaJump && form.jump_metodo === "chiave" ? form.jump_chiave : null,
      });
      caricaSessioni();
    }

    tabs.push(tab);
    tabAttivoId = tab.id;
    mostraForm = false;
  }

  async function chiudiTab(id) {
    const t = tabs.find((x) => x.id === id);
    for (const p of t?.panes ?? []) {
      await api.termChiudi(p.pid).catch(() => {});
      delete refsTerm[p.pid];
    }
    const i = tabs.findIndex((x) => x.id === id);
    tabs = tabs.filter((x) => x.id !== id);
    if (tabAttivoId === id) {
      tabAttivoId = tabs[Math.max(0, i - 1)]?.id ?? null;
    }
  }

  // Divide la scheda attiva aggiungendo un pannello (stessa connessione).
  function split(direzione) {
    if (!tabAttivo || tabAttivo.panes.length >= 4) return;
    tabAttivo.layout = direzione;
    tabAttivo.panes.push({ pid: crypto.randomUUID(), connesso: false });
  }

  // Chiude un singolo pannello di una scheda.
  async function chiudiPane(tab, pid) {
    await api.termChiudi(pid).catch(() => {});
    delete refsTerm[pid];
    tab.panes = tab.panes.filter((p) => p.pid !== pid);
    if (tab.panes.length === 0) chiudiTab(tab.id);
    else if (tab.panes.length === 1) tab.layout = "singolo";
  }

  // Stile CSS della griglia dei pannelli in base a layout e numero.
  function grigliaPannelli(tab) {
    const n = tab.panes.length;
    if (n <= 1) return "";
    if (tab.layout === "v") return `grid-template-rows: repeat(${n}, 1fr)`;
    return `grid-template-columns: repeat(${n}, 1fr)`;
  }

  // Inoltra l'input: a un solo pannello o a tutti (broadcast).
  function invia(pid, dati) {
    if (impostazioni.broadcast) {
      for (const t of tabs)
        for (const p of t.panes) if (p.connesso) api.termScrivi(p.pid, dati);
    } else {
      api.termScrivi(pid, dati);
    }
  }

  // Invia un comando (snippet) al terminale attivo, con a capo finale.
  function inviaSnippet(comando) {
    if (tabAttivo) api.termScrivi(tabAttivo.id, comando + "\n");
  }

  // Apre una nuova scheda con gli stessi parametri di una esistente.
  function duplica(t) {
    const nid = crypto.randomUUID();
    const copia = {
      ...t,
      id: nid,
      layout: "singolo",
      panes: [{ pid: nid, connesso: false }],
    };
    tabs.push(copia);
    tabAttivoId = nid;
  }

  // Azioni sui pannelli della scheda attiva (delegano ai componenti via ref).
  const azione = (fn, ...args) => {
    if (!tabAttivo) return;
    for (const p of tabAttivo.panes) {
      const r = refsTerm[p.pid];
      if (r && r[fn]) r[fn](...args);
    }
  };

  // Attiva/disattiva la registrazione su file della scheda attiva.
  async function toggleLog() {
    if (!tabAttivo) return;
    const pid = tabAttivo.panes[0].pid;
    if (tabAttivo.logAttivo) {
      await api.termLogFerma(pid).catch(() => {});
      tabAttivo.logAttivo = false;
    } else {
      const dest = await salvaFile({ defaultPath: `${tabAttivo.nome}.log` });
      if (!dest) return;
      try {
        await api.termLogAvvia(pid, dest);
        tabAttivo.logAttivo = true;
      } catch (e) {
        alert(e);
      }
    }
  }

  // Conferma la chiave del server e ritenta la connessione.
  function accettaHostKey() {
    const modo = hostKey.stato === "cambiata" ? "sostituisci" : "accetta";
    refsTerm[hostKey.id]?.riprovaConFiducia(modo);
    hostKey = null;
  }

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

<svelte:window onkeydown={scorciatoie} />

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
          style={t.colore ? `border-top:2px solid ${t.colore}` : ""}
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
        {#if tabAttivo.tipo !== "vnc"}
          {#if tabAttivo.panes.some((p) => !p.connesso)}
            <button class="strumento" title="Riconnetti" onclick={() => azione("riconnetti")}>↻ Riconnetti</button>
          {/if}
          <button class="strumento" title="Pulisci schermo" onclick={() => azione("pulisci")}>🧹</button>
          <button class="strumento" title="Riduci testo" onclick={() => azione("zoom", -1)}>A−</button>
          <button class="strumento" title="Ingrandisci testo" onclick={() => azione("zoom", 1)}>A+</button>
          <button
            class="strumento {tabAttivo.logAttivo ? 'attivo-log' : ''}"
            title="Registra su file"
            onclick={toggleLog}>{tabAttivo.logAttivo ? "⏺ Log ON" : "⏺ Log"}</button
          >
        {/if}
        <button class="strumento" title="Dividi affiancato" onclick={() => split("h")}>▦</button>
        <button class="strumento" title="Dividi impilato" onclick={() => split("v")}>▤</button>
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
          <div class="area-terminali" style={grigliaPannelli(t)}>
            {#each t.panes as p (p.pid)}
              <div class="term-wrap">
                {#if t.panes.length > 1}
                  <button class="chiudi-pane" title="Chiudi pannello" onclick={() => chiudiPane(t, p.pid)}>✕</button>
                {/if}
                {#if t.tipo === "vnc"}
                  <VncView
                    tab={{ ...t, id: p.pid }}
                    onConnesso={() => (p.connesso = true)}
                    onChiuso={() => (p.connesso = false)}
                  />
                {:else}
                  <Terminale
                    bind:this={refsTerm[p.pid]}
                    tab={{ ...t, id: p.pid }}
                    attivo={t.id === tabAttivoId}
                    {invia}
                    onConnesso={() => (p.connesso = true)}
                    onChiuso={() => (p.connesso = false)}
                    onHostKey={(info) => (hostKey = info)}
                  />
                {/if}
              </div>
            {/each}
          </div>
          {#if t.tipo === "ssh"}
            <Sftp
              id={t.panes[0].pid}
              pronto={t.panes[0].connesso}
              attivo={t.id === tabAttivoId}
            />
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

{#if mostraPalette}
  <PaletteComandi
    {sessioni}
    onApri={(s) => ((mostraPalette = false), apriSalvata(s))}
    onNuova={() => ((mostraPalette = false), nuovaConnessione())}
    onChiudi={() => (mostraPalette = false)}
  />
{/if}

<CodaTrasferimenti />

{#if hostKey}
  <div class="overlay" onclick={() => (hostKey = null)}>
    <div class="modale" onclick={(e) => e.stopPropagation()}>
      <h2>
        {hostKey.stato === "cambiata" ? "⚠️ Chiave del server CAMBIATA" : "🔑 Server sconosciuto"}
      </h2>
      {#if hostKey.stato === "cambiata"}
        <p style="color:#ff8787">
          La chiave del server è diversa da quella memorizzata! Potrebbe essere un attacco
          (man-in-the-middle), oppure il server è stato reinstallato. Procedi solo se sai perché.
        </p>
      {:else}
        <p style="color:var(--testo2)">
          È la prima volta che ti colleghi a questo server. Verifica che l'impronta corrisponda
          a quella attesa prima di fidarti.
        </p>
      {/if}
      <div class="campo">
        <label>Impronta (SHA256)</label>
        <input readonly value={hostKey.impronta} />
      </div>
      <div class="pulsanti">
        <button onclick={() => (hostKey = null)}>Annulla</button>
        <button class="primario" onclick={accettaHostKey}>
          {hostKey.stato === "cambiata" ? "Sostituisci e connetti" : "Fidati e connetti"}
        </button>
      </div>
    </div>
  </div>
{/if}
