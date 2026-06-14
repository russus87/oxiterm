<script>
  // Anteprima rapida di un file remoto (testo o immagine) senza scaricarlo.
  import { onMount } from "svelte";
  import * as api from "../lib/api.js";

  let { id, percorso, nome, onChiudi } = $props();

  let tipo = $state("caricamento"); // caricamento | testo | img | errore
  let testo = $state("");
  let src = $state("");
  let errore = $state("");

  const IMMAGINI = ["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "ico"];

  onMount(async () => {
    const ext = (nome.split(".").pop() || "").toLowerCase();
    try {
      if (IMMAGINI.includes(ext)) {
        const b64 = await api.sftpLeggiBase64(id, percorso);
        const mime =
          ext === "svg" ? "image/svg+xml" : ext === "jpg" ? "image/jpeg" : `image/${ext}`;
        src = `data:${mime};base64,${b64}`;
        tipo = "img";
      } else {
        testo = await api.sftpLeggiTesto(id, percorso);
        tipo = "testo";
      }
    } catch (e) {
      errore = String(e);
      tipo = "errore";
    }
  });
</script>

<div class="overlay" onclick={onChiudi}>
  <div class="modale" onclick={(e) => e.stopPropagation()} style="width:760px;max-width:92vw">
    <h2 style="font-size:14px">👁 {nome}</h2>
    {#if tipo === "caricamento"}
      <div class="vuoto">Caricamento…</div>
    {:else if tipo === "img"}
      <div style="text-align:center;max-height:60vh;overflow:auto;background:#111;padding:8px;border-radius:6px">
        <img {src} alt={nome} style="max-width:100%;height:auto" />
      </div>
    {:else if tipo === "testo"}
      <pre style="max-height:60vh;overflow:auto;background:var(--sfondo);padding:10px;border-radius:6px;font-size:12px;white-space:pre-wrap;word-break:break-word">{testo}</pre>
    {:else}
      <div class="vuoto" style="color:#ff8787">{errore}</div>
    {/if}
    <div class="pulsanti">
      <button class="primario" onclick={onChiudi}>Chiudi</button>
    </div>
  </div>
</div>
