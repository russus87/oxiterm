// Impostazioni dell'app, persistite in localStorage. Usa le rune di Svelte 5:
// `impostazioni` è uno stato reattivo globale che i componenti possono leggere
// e modificare; un $effect lo salva automaticamente a ogni cambiamento.

const CHIAVE = "oxiterm-impostazioni";

const DEFAULT = {
  fontFamily: "Consolas, 'DejaVu Sans Mono', monospace",
  fontSize: 14,
  tema: "Scuro",
  cursorBlink: true,
  scrollback: 5000,
  broadcast: false, // invia l'input a tutte le schede contemporaneamente
  autoReconnect: true, // riconnetti automaticamente le sessioni cadute
};

function carica() {
  try {
    return { ...DEFAULT, ...JSON.parse(localStorage.getItem(CHIAVE) || "{}") };
  } catch {
    return { ...DEFAULT };
  }
}

export const impostazioni = $state(carica());

// Salva automaticamente quando qualcosa cambia.
$effect.root(() => {
  $effect(() => {
    localStorage.setItem(CHIAVE, JSON.stringify(impostazioni));
  });
});
