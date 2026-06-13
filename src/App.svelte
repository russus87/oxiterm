<script>
  // Componente principale: sidebar col session manager + area a schede con
  // terminale SSH e pannello SFTP per ogni sessione aperta.
  import { onMount } from "svelte";
  import { open as apriFile } from "@tauri-apps/plugin-dialog";
  import * as api from "./lib/api.js";
  import Terminale from "./components/Terminale.svelte";
  import Sftp from "./components/Sftp.svelte";

  let sessioni = $state([]); // rubrica salvata
  let tabs = $state([]); // sessioni aperte
  let tabAttivoId = $state(null);

  // Stato del form "nuova connessione".
  let mostraForm = $state(false);
  let form = $state(vuotoForm());

  function vuotoForm() {
    return {
      idSalvata: null,
      nome: "",
      host: "",
      porta: 22,
      utente: "",
      metodo: "password", // password | chiave
      password: "",
      percorsoChiave: "",
      passphrase: "",
      salva: true,
    };
  }

  onMount(caricaSessioni);

  async function caricaSessioni() {
    try {
      sessioni = await api.listaSessioni();
    } catch (e) {
      console.error(e);
    }
  }

  function nuovaConnessione() {
    form = vuotoForm();
    mostraForm = true;
  }

  // Apre il form precompilato da una sessione salvata (manca solo il segreto).
  function apriSalvata(s) {
    form = {
      idSalvata: s.id,
      nome: s.nome,
      host: s.host,
      porta: s.porta,
      utente: s.utente,
      metodo: s.chiave ? "chiave" : "password",
      password: "",
      percorsoChiave: s.chiave || "",
      passphrase: "",
      salva: false,
    };
    mostraForm = true;
  }

  async function scegliChiave() {
    const f = await apriFile({ multiple: false });
    if (f) form.percorsoChiave = f;
  }

  async function eliminaSalvata(s) {
    if (!confirm(`Eliminare la sessione salvata "${s.nome}"?`)) return;
    await api.eliminaSessione(s.id);
    caricaSessioni();
  }

  async function connetti() {
    const auth =
      form.metodo === "password"
        ? { tipo: "password", password: form.password }
        : {
            tipo: "chiave",
            percorso: form.percorsoChiave,
            passphrase: form.passphrase || null,
          };

    const nome = form.nome || `${form.utente}@${form.host}`;
    const tab = {
      id: crypto.randomUUID(),
      nome,
      host: form.host,
      porta: Number(form.porta),
      utente: form.utente,
      auth,
      connesso: false,
    };

    // Salva nella rubrica se richiesto (senza segreti).
    if (form.salva) {
      await api.salvaSessione({
        id: form.idSalvata || crypto.randomUUID(),
        nome,
        host: form.host,
        porta: Number(form.porta),
        utente: form.utente,
        chiave: form.metodo === "chiave" ? form.percorsoChiave : null,
      });
      caricaSessioni();
    }

    tabs.push(tab);
    tabAttivoId = tab.id;
    mostraForm = false;
  }

  async function chiudiTab(id) {
    await api.sshDisconnetti(id).catch(() => {});
    const i = tabs.findIndex((t) => t.id === id);
    tabs = tabs.filter((t) => t.id !== id);
    if (tabAttivoId === id) {
      tabAttivoId = tabs[Math.max(0, i - 1)]?.id ?? null;
    }
  }
</script>

<div class="app">
  <!-- Sidebar: session manager -->
  <aside class="sidebar">
    <h1><span class="pallino"></span> Oxiterm</h1>
    <div class="azioni">
      <button class="primario" onclick={nuovaConnessione}>+ Nuova sessione</button>
    </div>
    <div class="lista-sessioni">
      {#each sessioni as s (s.id)}
        <div class="voce-sessione" onclick={() => apriSalvata(s)}>
          <span class="nome">{s.nome}</span>
          <span class="dett">{s.utente}@{s.host}:{s.porta}</span>
          <button
            class="x pericolo"
            title="Elimina"
            onclick={(e) => (e.stopPropagation(), eliminaSalvata(s))}>elimina</button
          >
        </div>
      {:else}
        <div class="vuoto">Nessuna sessione salvata.<br />Creane una con "+ Nuova sessione".</div>
      {/each}
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
          <span>{t.nome}</span>
          <button class="x" onclick={(e) => (e.stopPropagation(), chiudiTab(t.id))}>✕</button>
        </div>
      {/each}
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
        <div class="pannello-tab {t.id === tabAttivoId ? '' : 'nascosto'}">
          <div class="term-wrap">
            <Terminale
              tab={t}
              attivo={t.id === tabAttivoId}
              onConnesso={() => (t.connesso = true)}
              onChiuso={() => (t.connesso = false)}
            />
          </div>
          <Sftp id={t.id} pronto={t.connesso} />
        </div>
      {/each}
    </div>
  </main>
</div>

<!-- Form nuova connessione -->
{#if mostraForm}
  <div class="overlay" onclick={() => (mostraForm = false)}>
    <div class="modale" onclick={(e) => e.stopPropagation()}>
      <h2>Nuova connessione SSH</h2>
      <div class="campo">
        <label>Nome (facoltativo)</label>
        <input bind:value={form.nome} placeholder="Server di produzione" />
      </div>
      <div class="riga">
        <div class="campo" style="flex:3">
          <label>Host</label>
          <input bind:value={form.host} placeholder="192.168.1.10 o esempio.com" />
        </div>
        <div class="campo" style="flex:1">
          <label>Porta</label>
          <input type="number" bind:value={form.porta} />
        </div>
      </div>
      <div class="campo">
        <label>Utente</label>
        <input bind:value={form.utente} placeholder="root" />
      </div>
      <div class="campo">
        <label>Autenticazione</label>
        <select bind:value={form.metodo}>
          <option value="password">Password</option>
          <option value="chiave">Chiave privata</option>
        </select>
      </div>
      {#if form.metodo === "password"}
        <div class="campo">
          <label>Password</label>
          <input type="password" bind:value={form.password} />
        </div>
      {:else}
        <div class="campo">
          <label>File chiave privata</label>
          <div class="riga">
            <input bind:value={form.percorsoChiave} placeholder="~/.ssh/id_ed25519" />
            <button onclick={scegliChiave}>Sfoglia</button>
          </div>
        </div>
        <div class="campo">
          <label>Passphrase (se presente)</label>
          <input type="password" bind:value={form.passphrase} />
        </div>
      {/if}
      <div class="campo" style="display:flex;align-items:center;gap:8px">
        <input type="checkbox" bind:checked={form.salva} style="width:auto" id="salva" />
        <label for="salva" style="margin:0">Salva nella rubrica</label>
      </div>
      <div class="pulsanti">
        <button onclick={() => (mostraForm = false)}>Annulla</button>
        <button class="primario" onclick={connetti} disabled={!form.host || !form.utente}>
          Connetti
        </button>
      </div>
    </div>
  </div>
{/if}
