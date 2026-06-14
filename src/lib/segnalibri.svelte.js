// Segnalibri SFTP: percorsi preferiti, salvati nel backend (così rientrano
// nella sincronizzazione cloud insieme alla rubrica).
import * as api from "./api.js";

export const segnalibri = $state([]);
let caricato = false;

// Da chiamare una volta all'avvio dell'app.
export async function caricaSegnalibri() {
  try {
    const l = await api.listaSegnalibri();
    segnalibri.splice(0, segnalibri.length, ...l);
  } catch {}
  caricato = true;
}

// Salva sul backend a ogni modifica (dopo il caricamento iniziale).
$effect.root(() => {
  $effect(() => {
    const copia = [...segnalibri];
    if (caricato) api.salvaSegnalibri(copia).catch(() => {});
  });
});

export function aggiungi(percorso) {
  if (percorso && !segnalibri.includes(percorso)) segnalibri.push(percorso);
}

export function rimuovi(percorso) {
  const i = segnalibri.indexOf(percorso);
  if (i >= 0) segnalibri.splice(i, 1);
}
