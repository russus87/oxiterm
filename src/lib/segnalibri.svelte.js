// Segnalibri SFTP: percorsi preferiti, salvati in localStorage.
const CHIAVE = "oxiterm-segnalibri";

function carica() {
  try {
    return JSON.parse(localStorage.getItem(CHIAVE) || "[]");
  } catch {
    return [];
  }
}

export const segnalibri = $state(carica());

$effect.root(() => {
  $effect(() => {
    localStorage.setItem(CHIAVE, JSON.stringify(segnalibri));
  });
});

export function aggiungi(percorso) {
  if (percorso && !segnalibri.includes(percorso)) segnalibri.push(percorso);
}

export function rimuovi(percorso) {
  const i = segnalibri.indexOf(percorso);
  if (i >= 0) segnalibri.splice(i, 1);
}
