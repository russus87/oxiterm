<script>
  // Palette comandi (Ctrl+P): cerca rapidamente una sessione salvata e aprila.
  let { sessioni, onApri, onNuova, onChiudi } = $props();

  let query = $state("");
  let evidenziato = $state(0);

  const filtrate = $derived(
    sessioni.filter((s) => {
      const t = `${s.nome} ${s.host ?? ""} ${s.utente ?? ""} ${s.gruppo ?? ""} ${(s.tags ?? []).join(" ")}`.toLowerCase();
      return t.includes(query.toLowerCase());
    }),
  );

  function tasto(e) {
    if (e.key === "ArrowDown") evidenziato = Math.min(filtrate.length - 1, evidenziato + 1);
    else if (e.key === "ArrowUp") evidenziato = Math.max(0, evidenziato - 1);
    else if (e.key === "Enter") {
      if (filtrate[evidenziato]) onApri(filtrate[evidenziato]);
      else onNuova();
    } else if (e.key === "Escape") onChiudi();
  }
</script>

<div class="overlay" onclick={onChiudi}>
  <div class="palette" onclick={(e) => e.stopPropagation()}>
    <!-- svelte-ignore a11y_autofocus -->
    <input
      autofocus
      placeholder="Cerca una sessione… (Invio per aprire, Esc per chiudere)"
      bind:value={query}
      onkeydown={tasto}
      oninput={() => (evidenziato = 0)}
    />
    <div class="palette-lista">
      {#each filtrate as s, i (s.id)}
        <div
          class="palette-voce {i === evidenziato ? 'sel' : ''}"
          onclick={() => onApri(s)}
          onmouseenter={() => (evidenziato = i)}
        >
          <span class="nome">{s.nome}</span>
          <span class="dett">{s.tipo ?? "ssh"} · {s.host ?? "locale"}</span>
        </div>
      {:else}
        <div class="palette-voce" onclick={onNuova}>➕ Nuova sessione…</div>
      {/each}
    </div>
  </div>
</div>
