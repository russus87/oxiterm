<script>
  // Pannello SFTP affiancato al terminale: sfoglia, scarica e carica file remoti.
  import { open as apriFile, save as salvaFile } from "@tauri-apps/plugin-dialog";
  import { openPath } from "@tauri-apps/plugin-opener";
  import * as api from "../lib/api.js";

  let { id, pronto } = $props();

  let percorso = $state("");
  let modificaPercorso = $state(""); // valore della barra modificabile
  let voci = $state([]);
  let errore = $state("");
  let stato = $state(""); // messaggio durante i trasferimenti
  let caricato = false;

  function unisci(base, nome) {
    return base.endsWith("/") ? base + nome : base + "/" + nome;
  }

  function genitore(p) {
    if (p === "/" || !p.includes("/")) return "/";
    const tagliato = p.replace(/\/+$/, "");
    const i = tagliato.lastIndexOf("/");
    return i <= 0 ? "/" : tagliato.slice(0, i);
  }

  function formattaDim(n) {
    if (n < 1024) return n + " B";
    if (n < 1024 * 1024) return (n / 1024).toFixed(1) + " KB";
    return (n / 1024 / 1024).toFixed(1) + " MB";
  }

  // Segmenti del percorso per il breadcrumb: [{nome, percorso}].
  const briciole = $derived.by(() => {
    const parti = percorso.split("/").filter(Boolean);
    let acc = "";
    const out = [{ nome: "/", percorso: "/" }];
    for (const p of parti) {
      acc += "/" + p;
      out.push({ nome: p, percorso: acc });
    }
    return out;
  });

  async function vai(p) {
    errore = "";
    try {
      voci = await api.sftpLista(id, p);
      percorso = p;
      modificaPercorso = p;
    } catch (e) {
      errore = String(e);
    }
  }

  async function aggiorna() {
    if (percorso) await vai(percorso);
  }

  $effect(() => {
    if (pronto && !caricato) {
      caricato = true;
      api
        .sftpHome(id)
        .then((h) => vai(h || "."))
        .catch((e) => (errore = String(e)));
    }
  });

  async function apri(v) {
    if (v.dir) vai(unisci(percorso, v.nome));
  }

  async function scarica(v) {
    const dest = await salvaFile({ defaultPath: v.nome });
    if (!dest) return;
    stato = `⬇ scarico ${v.nome}…`;
    try {
      await api.sftpScarica(id, unisci(percorso, v.nome), dest);
      stato = `✓ scaricato ${v.nome}`;
    } catch (e) {
      errore = String(e);
      stato = "";
    }
  }

  async function carica() {
    const sorgente = await apriFile({ multiple: false });
    if (!sorgente) return;
    const nome = sorgente.split(/[\\/]/).pop();
    stato = `⬆ carico ${nome}…`;
    try {
      await api.sftpCarica(id, sorgente, unisci(percorso, nome));
      stato = `✓ caricato ${nome}`;
      aggiorna();
    } catch (e) {
      errore = String(e);
      stato = "";
    }
  }

  async function nuovaCartella() {
    const nome = prompt("Nome della nuova cartella:");
    if (!nome) return;
    try {
      await api.sftpCreaCartella(id, unisci(percorso, nome));
      aggiorna();
    } catch (e) {
      errore = String(e);
    }
  }

  async function elimina(v) {
    if (!confirm(`Eliminare "${v.nome}"?`)) return;
    try {
      await api.sftpElimina(id, unisci(percorso, v.nome), v.dir);
      aggiorna();
    } catch (e) {
      errore = String(e);
    }
  }

  // Apre il file remoto nell'editor di sistema; le modifiche salvate vengono
  // ricaricate automaticamente sul server (gestito dal backend).
  async function modifica(v) {
    stato = `✎ apro ${v.nome}…`;
    try {
      const locale = await api.sftpApriEditor(id, unisci(percorso, v.nome));
      await openPath(locale);
      stato = `✎ ${v.nome} aperto (auto-salvataggio attivo)`;
    } catch (e) {
      errore = String(e);
      stato = "";
    }
  }

  async function rinomina(v) {
    const nuovo = prompt("Nuovo nome:", v.nome);
    if (!nuovo || nuovo === v.nome) return;
    try {
      await api.sftpRinomina(id, unisci(percorso, v.nome), unisci(percorso, nuovo));
      aggiorna();
    } catch (e) {
      errore = String(e);
    }
  }
</script>

<div class="sftp">
  <div class="barra">
    <button title="Su" onclick={() => vai(genitore(percorso))}>↑</button>
    <button title="Aggiorna" onclick={aggiorna}>⟳</button>
    <button title="Nuova cartella" onclick={nuovaCartella}>＋📁</button>
    <button title="Carica file" onclick={carica}>⬆ Carica</button>
  </div>

  <!-- Breadcrumb cliccabile -->
  <div class="briciole">
    {#each briciole as b, i (b.percorso)}
      <span class="briciola" onclick={() => vai(b.percorso)}>{b.nome}</span>{#if i < briciole.length - 1}<span class="sep">/</span>{/if}
    {/each}
  </div>

  <!-- Percorso modificabile -->
  <input
    class="barra-percorso"
    bind:value={modificaPercorso}
    onkeydown={(e) => e.key === "Enter" && vai(modificaPercorso)}
    placeholder="/percorso/remoto"
  />

  {#if stato}<div class="stato">{stato}</div>{/if}
  {#if errore}<div class="vuoto" style="color:#ff8787">{errore}</div>{/if}

  <div class="lista">
    {#each voci as v (v.nome)}
      <div class="riga-file" ondblclick={() => apri(v)}>
        <span class="icona">{v.dir ? "📁" : "📄"}</span>
        <span class="nome">{v.nome}</span>
        {#if !v.dir}<span class="dim">{formattaDim(v.dimensione)}</span>{/if}
        <span class="ops">
          {#if !v.dir}
            <button title="Apri in editor" onclick={(e) => (e.stopPropagation(), modifica(v))}>✏️</button>
            <button title="Scarica" onclick={(e) => (e.stopPropagation(), scarica(v))}>⬇</button>
          {/if}
          <button title="Rinomina" onclick={(e) => (e.stopPropagation(), rinomina(v))}>✎</button>
          <button class="pericolo" title="Elimina" onclick={(e) => (e.stopPropagation(), elimina(v))}>✕</button>
        </span>
      </div>
    {/each}
    {#if pronto && voci.length === 0 && !errore}
      <div class="vuoto">Cartella vuota</div>
    {/if}
  </div>
</div>
