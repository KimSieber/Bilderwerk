import { invoke } from "@tauri-apps/api/core";
import type {
  CacheInfo,
  DbInfo,
  Details,
  DropboxStatus,
  Einstellungen,
  EinstellungenAenderung,
  Ergebnis,
  Facette,
  Regel,
  SuchFilter,
} from "./types";
import type { Antwort } from "./vorschau";

export const api = {
  start: () => invoke<DbInfo>("start"),
  dbSetzen: (pfad: string) => invoke<DbInfo>("db_setzen", { pfad }),
  /** DB neu öffnen; die App erfährt es über das Ereignis „db-neu“ (v0.0.7) */
  dbNeuLaden: () => invoke<DbInfo>("db_neu_laden"),
  suchen: (filter: SuchFilter) => invoke<Ergebnis>("suchen", { filter }),
  facetten: (filter: SuchFilter) => invoke<Facette[]>("facetten", { filter }),
  dropboxStatus: () => invoke<DropboxStatus>("dropbox_status"),
  dropboxAnmelden: () => invoke<DropboxStatus>("dropbox_anmelden"),
  dropboxAbmelden: () => invoke<DropboxStatus>("dropbox_abmelden"),
  cacheInfo: () => invoke<CacheInfo>("cache_info"),
  cacheLeeren: () => invoke<CacheInfo>("cache_leeren"),
  // Einstellungsdialog (v0.3)
  einstellungen: () => invoke<Einstellungen>("einstellungen"),
  einstellungenSetzen: (aenderung: EinstellungenAenderung) => invoke<Einstellungen>("einstellungen_setzen", { aenderung }),
  anbieterVorschlaege: () => invoke<Regel[]>("anbieter_vorschlaege"),
  // Detailbereich (v0.0.6)
  details: (id: number) => invoke<Details>("details", { id }),
  vorschauGross: (pfad: string) => invoke<Antwort>("vorschau_gross", { pfad }),
  originalOeffnen: (pfad: string) => invoke<void>("original_oeffnen", { pfad }),
  imFinderZeigen: (pfad: string) => invoke<void>("im_finder_zeigen", { pfad }),
  dateinameVorschlag: (pfad: string) => invoke<string>("dateiname_vorschlag", { pfad }),
  sichernUnter: (pfad: string, ziel: string) => invoke<string>("sichern_unter", { pfad, ziel }),
  /** knopf = [x, y, breite, höhe] des Teilen-Knopfs im Fenster */
  teilen: (pfad: string, knopf: [number, number, number, number]) => invoke<void>("teilen", { pfad, knopf }),
};

/** "2015-06-09T14:12:36" → "09.06.2015"; unplausible Werte bleiben roh. */
export function datumDe(iso: string | null): string {
  if (!iso) return "";
  const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(iso);
  if (!m) return iso;
  const [, j, mo, t] = m;
  if (+mo < 1 || +mo > 12 || +t < 1 || +t > 31) return iso.slice(0, 10);
  return `${t}.${mo}.${j}`;
}

export function zahlDe(n: number): string {
  return n.toLocaleString("de-DE");
}

/** Bytes lesbar: 1,3 GB · 840 MB · 12 KB */
export function groesseDe(bytes: number): string {
  const e = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let n = bytes;
  while (n >= 1024 && i < e.length - 1) {
    n /= 1024;
    i++;
  }
  const z = i >= 3 ? n.toLocaleString("de-DE", { maximumFractionDigits: 1 }) : Math.round(n).toLocaleString("de-DE");
  return `${z} ${e[i]}`;
}
