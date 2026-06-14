// Controllo aggiornamenti tramite l'updater di Tauri (artefatti sul repo
// pubblico oxiterm-dist). In dev non è disponibile: si ignora silenziosamente.
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export async function controllaAggiornamenti(silenzioso = false) {
  try {
    const update = await check();
    if (update) {
      if (
        confirm(
          `È disponibile Oxiterm ${update.version}.\nScaricare e installare ora? L'app verrà riavviata.`,
        )
      ) {
        await update.downloadAndInstall();
        await relaunch();
      }
    } else if (!silenzioso) {
      alert("Sei già all'ultima versione.");
    }
  } catch (e) {
    if (!silenzioso) alert("Controllo aggiornamenti non riuscito: " + e);
  }
}
