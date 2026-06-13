<script>
  // Visore VNC (sperimentale): disegna i frame ricevuti dal backend su un canvas
  // e inoltra mouse e tastiera al server.
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import * as api from "../lib/api.js";

  let { tab, onConnesso, onChiuso } = $props();

  let canvas;
  let ctx;
  let off = [];
  let bottoni = 0; // maschera bottoni mouse premuti
  let errore = $state("");

  // Mappa dei tasti speciali verso keysym X11.
  const KEYSYM = {
    Enter: 0xff0d,
    Backspace: 0xff08,
    Tab: 0xff09,
    Escape: 0xff1b,
    Delete: 0xffff,
    ArrowLeft: 0xff51,
    ArrowUp: 0xff52,
    ArrowRight: 0xff53,
    ArrowDown: 0xff54,
    Home: 0xff50,
    End: 0xff57,
    PageUp: 0xff55,
    PageDown: 0xff56,
    Shift: 0xffe1,
    Control: 0xffe3,
    Alt: 0xffe9,
    " ": 0x20,
  };

  function keysym(e) {
    if (KEYSYM[e.key] !== undefined) return KEYSYM[e.key];
    if (e.key.length === 1) return e.key.codePointAt(0);
    return 0;
  }

  // Converte le coordinate del mouse nel sistema del server remoto.
  function coord(e) {
    const r = canvas.getBoundingClientRect();
    const x = Math.round(((e.clientX - r.left) / r.width) * canvas.width);
    const y = Math.round(((e.clientY - r.top) / r.height) * canvas.height);
    return { x: Math.max(0, x), y: Math.max(0, y) };
  }

  onMount(async () => {
    ctx = canvas.getContext("2d");

    off.push(
      await listen(`vnc-frame-${tab.id}`, (e) => {
        const f = e.payload;
        if (f.resize) {
          canvas.width = f.resize[0];
          canvas.height = f.resize[1];
          return;
        }
        // Decodifica base64 -> RGBA -> disegno del rettangolo.
        const bin = atob(f.dati);
        const arr = new Uint8ClampedArray(bin.length);
        for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
        if (arr.length >= f.w * f.h * 4) {
          ctx.putImageData(new ImageData(arr, f.w, f.h), f.x, f.y);
        }
      }),
    );
    off.push(
      await listen(`vnc-chiuso-${tab.id}`, () => {
        errore = "Sessione VNC chiusa.";
        onChiuso?.();
      }),
    );

    try {
      await api.apriVnc(tab.id, tab.host, tab.porta, tab.vnc_password);
      onConnesso?.();
    } catch (e) {
      errore = String(e);
    }
  });

  function muovi(e) {
    const { x, y } = coord(e);
    api.vncMouse(tab.id, x, y, bottoni);
  }
  function giu(e) {
    bottoni |= 1 << e.button;
    muovi(e);
  }
  function su(e) {
    bottoni &= ~(1 << e.button);
    muovi(e);
  }
  function rotella(e) {
    e.preventDefault();
    // Scroll = pressione momentanea dei bottoni 4 (su) / 5 (giù).
    const bit = e.deltaY < 0 ? 1 << 3 : 1 << 4;
    const { x, y } = coord(e);
    api.vncMouse(tab.id, x, y, bottoni | bit);
    api.vncMouse(tab.id, x, y, bottoni);
  }
  function tastoGiu(e) {
    e.preventDefault();
    const k = keysym(e);
    if (k) api.vncTasto(tab.id, true, k);
  }
  function tastoSu(e) {
    e.preventDefault();
    const k = keysym(e);
    if (k) api.vncTasto(tab.id, false, k);
  }

  onDestroy(() => {
    off.forEach((f) => f());
    api.vncChiudi(tab.id).catch(() => {});
  });
</script>

<div class="vnc-wrap">
  {#if errore}<div class="vnc-errore">{errore}</div>{/if}
  <canvas
    bind:this={canvas}
    width="800"
    height="600"
    tabindex="0"
    onmousemove={muovi}
    onmousedown={giu}
    onmouseup={su}
    onwheel={rotella}
    onkeydown={tastoGiu}
    onkeyup={tastoSu}
    oncontextmenu={(e) => e.preventDefault()}
  ></canvas>
</div>
