<script>
  // Strumenti di rete locali: ping, traceroute, controllo porta, Wake-on-LAN.
  import * as api from "../lib/api.js";

  let { hostIniziale = "", onChiudi } = $props();

  let host = $state(hostIniziale);
  let porta = $state(22);
  let mac = $state("");
  let output = $state("");
  let occupato = $state(false);

  async function esegui(etichetta, azione) {
    occupato = true;
    output = `${etichetta}…`;
    try {
      output = await azione();
    } catch (e) {
      output = String(e);
    }
    occupato = false;
  }

  const ping = () => esegui("ping", () => api.netPing(host));
  const traceroute = () => esegui("traceroute", () => api.netTraceroute(host));
  const porta_ck = () =>
    esegui("controllo porta", async () => {
      const aperta = await api.netPorta(host, Number(porta));
      return aperta ? `✓ ${host}:${porta} è APERTA` : `✗ ${host}:${porta} chiusa o filtrata`;
    });
  const wol = () => esegui("Wake-on-LAN", async () => (await api.netWol(mac), "✓ Magic packet inviato"));
</script>

<div class="overlay" onclick={onChiudi}>
  <div class="modale" onclick={(e) => e.stopPropagation()} style="width:620px;max-width:94vw">
    <h2>Strumenti di rete</h2>

    <div class="riga">
      <div class="campo" style="flex:3">
        <label>Host</label>
        <input bind:value={host} placeholder="esempio.com o 192.168.1.10" />
      </div>
      <div class="campo" style="flex:1">
        <label>Porta</label>
        <input type="number" bind:value={porta} />
      </div>
    </div>
    <div class="riga">
      <button onclick={ping} disabled={occupato || !host}>Ping</button>
      <button onclick={traceroute} disabled={occupato || !host}>Traceroute</button>
      <button onclick={porta_ck} disabled={occupato || !host}>Controlla porta</button>
    </div>

    <div class="campo" style="margin-top:10px">
      <label>Wake-on-LAN (MAC)</label>
      <div class="riga">
        <input bind:value={mac} placeholder="AA:BB:CC:DD:EE:FF" />
        <button onclick={wol} disabled={occupato || !mac}>Accendi</button>
      </div>
    </div>

    <pre style="margin-top:12px;max-height:48vh;overflow:auto;background:var(--sfondo);padding:10px;border-radius:6px;font-size:12px;white-space:pre-wrap">{output}</pre>

    <div class="pulsanti">
      <button class="primario" onclick={onChiudi}>Chiudi</button>
    </div>
  </div>
</div>
