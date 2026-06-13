<script>
  // Riquadro fluttuante con i trasferimenti SFTP in corso e conclusi.
  import { trasferimenti, pulisci } from "../lib/trasferimenti.svelte.js";

  function perc(t) {
    if (!t.totale) return 0;
    return Math.min(100, Math.round((t.fatti / t.totale) * 100));
  }
</script>

{#if trasferimenti.length}
  <div class="coda">
    <div class="coda-testa">
      <b>Trasferimenti</b>
      <button class="x" onclick={pulisci} title="Pulisci conclusi">pulisci</button>
    </div>
    {#each trasferimenti as t (t.id)}
      <div class="coda-riga">
        <div class="coda-nome">
          {t.verso === "su" ? "⬆" : "⬇"} {t.nome}
          <span class="coda-stato">
            {#if t.stato === "fatto"}✓{:else if t.stato === "errore"}✕{:else}{perc(t)}%{/if}
          </span>
        </div>
        <div class="coda-barra">
          <div
            class="coda-avanz {t.stato}"
            style="width:{t.stato === 'fatto' ? 100 : perc(t)}%"
          ></div>
        </div>
      </div>
    {/each}
  </div>
{/if}
