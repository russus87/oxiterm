<script>
  // Form per aprire (e facoltativamente salvare) una connessione di qualsiasi tipo:
  // SSH, terminale locale, Telnet o seriale.
  import { onMount } from "svelte";
  import { open as apriFile } from "@tauri-apps/plugin-dialog";
  import * as api from "../lib/api.js";

  // `iniziale` precompila il form (da una sessione salvata). `onConnetti` riceve
  // l'oggetto form; `onChiudi` chiude il modale.
  let { iniziale = null, onConnetti, onChiudi } = $props();

  let form = $state(iniziale ? { ...vuoto(), ...iniziale } : vuoto());
  let porteSeriali = $state([]);

  function vuoto() {
    return {
      idSalvata: null,
      tipo: "ssh",
      nome: "",
      host: "",
      porta: 22,
      utente: "",
      metodo: "password",
      password: "",
      percorsoChiave: "",
      passphrase: "",
      shell: "",
      porta_seriale: "",
      baud: 115200,
      gruppo: "",
      colore: "#37b24d",
      salva: true,
      // VNC
      vnc_password: "",
      // Jump host (ProxyJump) — solo per SSH
      usaJump: false,
      jump_host: "",
      jump_porta: 22,
      jump_utente: "",
      jump_metodo: "password",
      jump_password: "",
      jump_chiave: "",
      jump_passphrase: "",
    };
  }

  onMount(async () => {
    try {
      porteSeriali = await api.porteSeriali();
    } catch {}
  });

  // Aggiusta la porta di default quando cambia il tipo.
  function cambiaTipo() {
    if (form.tipo === "ssh" && [23, 5900].includes(form.porta)) form.porta = 22;
    if (form.tipo === "telnet" && [22, 5900].includes(form.porta)) form.porta = 23;
    if (form.tipo === "vnc" && [22, 23].includes(form.porta)) form.porta = 5900;
  }

  async function scegliChiave() {
    const f = await apriFile({ multiple: false });
    if (f) form.percorsoChiave = f;
  }

  async function scegliChiaveJump() {
    const f = await apriFile({ multiple: false });
    if (f) form.jump_chiave = f;
  }

  const valido = $derived(
    (form.tipo === "ssh" && form.host && form.utente) ||
      (form.tipo === "telnet" && form.host) ||
      (form.tipo === "vnc" && form.host) ||
      (form.tipo === "seriale" && form.porta_seriale) ||
      form.tipo === "locale",
  );
</script>

<div class="overlay" onclick={onChiudi}>
  <div class="modale" onclick={(e) => e.stopPropagation()}>
    <h2>Nuova sessione</h2>

    <div class="campo">
      <label>Tipo</label>
      <select bind:value={form.tipo} onchange={cambiaTipo}>
        <option value="ssh">SSH</option>
        <option value="locale">Terminale locale</option>
        <option value="telnet">Telnet</option>
        <option value="seriale">Seriale</option>
        <option value="vnc">VNC (sperimentale)</option>
      </select>
    </div>

    <div class="campo">
      <label>Nome (facoltativo)</label>
      <input bind:value={form.nome} placeholder="Server di produzione" />
    </div>

    {#if form.tipo === "ssh" || form.tipo === "telnet" || form.tipo === "vnc"}
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
    {/if}

    {#if form.tipo === "vnc"}
      <div class="campo">
        <label>Password VNC (se richiesta)</label>
        <input type="password" bind:value={form.vnc_password} />
      </div>
    {/if}

    {#if form.tipo === "ssh"}
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

      <!-- Jump host / ProxyJump -->
      <div class="campo" style="display:flex;align-items:center;gap:8px;margin-top:6px">
        <input type="checkbox" bind:checked={form.usaJump} style="width:auto" id="usaJump" />
        <label for="usaJump" style="margin:0">Passa per un host intermedio (jump host)</label>
      </div>
      {#if form.usaJump}
        <div class="riga">
          <div class="campo" style="flex:3">
            <label>Jump host</label>
            <input bind:value={form.jump_host} placeholder="bastion.esempio.com" />
          </div>
          <div class="campo" style="flex:1">
            <label>Porta</label>
            <input type="number" bind:value={form.jump_porta} />
          </div>
        </div>
        <div class="campo">
          <label>Utente jump</label>
          <input bind:value={form.jump_utente} />
        </div>
        <div class="campo">
          <label>Autenticazione jump</label>
          <select bind:value={form.jump_metodo}>
            <option value="password">Password</option>
            <option value="chiave">Chiave privata</option>
          </select>
        </div>
        {#if form.jump_metodo === "password"}
          <div class="campo">
            <label>Password jump</label>
            <input type="password" bind:value={form.jump_password} />
          </div>
        {:else}
          <div class="campo">
            <label>Chiave privata jump</label>
            <div class="riga">
              <input bind:value={form.jump_chiave} placeholder="~/.ssh/id_ed25519" />
              <button onclick={scegliChiaveJump}>Sfoglia</button>
            </div>
          </div>
          <div class="campo">
            <label>Passphrase jump</label>
            <input type="password" bind:value={form.jump_passphrase} />
          </div>
        {/if}
      {/if}
    {/if}

    {#if form.tipo === "locale"}
      <div class="campo">
        <label>Shell (vuoto = predefinita del sistema)</label>
        <input bind:value={form.shell} placeholder="/bin/bash, powershell.exe…" />
      </div>
    {/if}

    {#if form.tipo === "seriale"}
      <div class="riga">
        <div class="campo" style="flex:2">
          <label>Porta</label>
          {#if porteSeriali.length}
            <select bind:value={form.porta_seriale}>
              <option value="" disabled selected>Scegli…</option>
              {#each porteSeriali as p}<option value={p}>{p}</option>{/each}
            </select>
          {:else}
            <input bind:value={form.porta_seriale} placeholder="/dev/ttyUSB0 o COM3" />
          {/if}
        </div>
        <div class="campo" style="flex:1">
          <label>Baud</label>
          <input type="number" bind:value={form.baud} />
        </div>
      </div>
    {/if}

    <div class="riga">
      <div class="campo" style="flex:3">
        <label>Gruppo (facoltativo)</label>
        <input bind:value={form.gruppo} placeholder="Es. Produzione, Casa…" />
      </div>
      <div class="campo" style="flex:1">
        <label>Colore</label>
        <input type="color" bind:value={form.colore} style="height:32px;padding:2px" />
      </div>
    </div>

    <div class="campo" style="display:flex;align-items:center;gap:8px">
      <input type="checkbox" bind:checked={form.salva} style="width:auto" id="salva" />
      <label for="salva" style="margin:0">Salva nella rubrica</label>
    </div>

    <div class="pulsanti">
      <button onclick={onChiudi}>Annulla</button>
      <button class="primario" onclick={() => onConnetti(form)} disabled={!valido}>
        Connetti
      </button>
    </div>
  </div>
</div>
