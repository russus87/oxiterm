// Notifiche desktop, rispettando l'impostazione dell'utente.
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { impostazioni } from "./impostazioni.svelte.js";

let permesso = null;

export async function notifica(titolo, corpo) {
  if (!impostazioni.notifiche) return;
  try {
    if (permesso === null) {
      permesso = await isPermissionGranted();
      if (!permesso) permesso = (await requestPermission()) === "granted";
    }
    if (permesso) sendNotification({ title: titolo, body: corpo });
  } catch {}
}
