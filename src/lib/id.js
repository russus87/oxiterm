// Genera un id univoco.
// ATTENZIONE: crypto.randomUUID() non è sempre disponibile nella webview di
// Tauri (su Linux/webkit2gtk l'origine tauri://localhost non è un "contesto
// sicuro"), quindi serve un fallback, altrimenti lancia e blocca tutto.
export function nuovoId() {
  try {
    if (globalThis.crypto && typeof crypto.randomUUID === "function") {
      return crypto.randomUUID();
    }
  } catch {}
  return (
    "id-" +
    Date.now().toString(36) +
    "-" +
    Math.random().toString(36).slice(2, 10)
  );
}
