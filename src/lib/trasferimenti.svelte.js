// Coda globale dei trasferimenti SFTP, con avanzamento in tempo reale.
// Il backend emette eventi "sftp-progresso" con { id, fatti, totale }.
import { listen } from "@tauri-apps/api/event";
import { notifica } from "./notifiche.js";

export const trasferimenti = $state([]); // { id, nome, verso, fatti, totale, stato }

export function aggiungi(id, nome, verso) {
  trasferimenti.push({ id, nome, verso, fatti: 0, totale: 0, stato: "in corso" });
}

export function completa(id, ok) {
  const t = trasferimenti.find((x) => x.id === id);
  if (t) {
    t.stato = ok ? "fatto" : "errore";
    notifica(
      ok ? "Trasferimento completato" : "Trasferimento fallito",
      t.nome,
    );
  }
}

export function pulisci() {
  // Rimuove i trasferimenti già conclusi.
  for (let i = trasferimenti.length - 1; i >= 0; i--) {
    if (trasferimenti[i].stato !== "in corso") trasferimenti.splice(i, 1);
  }
}

// Ascolta gli aggiornamenti di avanzamento (una sola volta, all'avvio).
listen("sftp-progresso", (e) => {
  const { id, fatti, totale } = e.payload;
  const t = trasferimenti.find((x) => x.id === id);
  if (t) {
    t.fatti = fatti;
    t.totale = totale;
  }
});
