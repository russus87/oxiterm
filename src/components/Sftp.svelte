<script>
  // Pannello SFTP affiancato al terminale: sfoglia, scarica e carica file remoti.
  import { open as apriFile, save as salvaFile } from "@tauri-apps/plugin-dialog";
  import * as api from "../lib/api.js";

  // `id` della sessione e flag che indica se la connessione è pronta.
  let { id, pronto } = $props();

  let percorso = $state("");
  let voci = $state([]);
  let errore = $state("");
  let caricato = false;

  // Unisce due pezzi di percorso in stile Unix.
  function unisci(base, nome) {
    if (base.endsWith("/")) return base + nome;
    return base + "/" + nome;
  }

  // Cartella genitore di un percorso assoluto.
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

  async function vai(p) {
    errore = "";
    try {
      voci = await api.sftpLista(id, p);
      percorso = p;
    } catch (e) {
      errore = String(e);
    }
  }

  async function aggiorna() {
    if (percorso) await vai(percorso);
  }

  // Al primo momento utile (connessione pronta) carica la home.
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
    try {
      await api.sftpScarica(id, unisci(percorso, v.nome), dest);
    } catch (e) {
      errore = String(e);
    }
  }

  async function carica() {
    const sorgente = await apriFile({ multiple: false });
    if (!sorgente) return;
    const nome = sorgente.split(/[\\/]/).pop();
    try {
      await api.sftpCarica(id, sorgente, unisci(percorso, nome));
      aggiorna();
    } catch (e) {
      errore = String(e);
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
  <div class="percorso">{percorso || "…"}</div>
  {#if errore}
    <div class="vuoto" style="color:#ff8787">{errore}</div>
  {/if}
  <div class="lista">
    {#each voci as v (v.nome)}
      <div class="riga-file" ondblclick={() => apri(v)}>
        <span class="icona">{v.dir ? "📁" : "📄"}</span>
        <span class="nome">{v.nome}</span>
        {#if !v.dir}<span class="dim">{formattaDim(v.dimensione)}</span>{/if}
        <span class="ops">
          {#if !v.dir}
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
