import { useEffect, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";

/** Kantenlänge der Vorschaubilder (Raster und Liste teilen sich denselben Cache). */
export const VORSCHAU_GROESSE = 640;

export type VorschauStatus =
  | { art: "bild"; url: string }
  | { art: "nurOnline" }
  | { art: "fehlt" }
  | { art: "fehler"; meldung: string }
  | { art: "laedt" };

export type Antwort =
  | { art: "bild"; datei: string }
  | { art: "datenbank"; schluessel: string }
  | { art: "nurOnline" }
  | { art: "fehlt" }
  | { art: "fehler"; meldung: string };

interface Auftrag {
  pfad: string;
  groesse: number;
  fertig: (s: VorschauStatus) => void;
  abgebrochen: boolean;
}

export const MAX_PARALLEL = 6;
const ergebnisse = new Map<string, VorschauStatus>();
const warteschlange: Auftrag[] = [];
let aktiv = 0;

const schluessel = (pfad: string, groesse: number) => `${groesse}|${pfad}`;

/** Antwort aus Rust → Anzeigestatus. Cache-Datei über das Asset-Protokoll, DB-Vorschau über `vorschau://`. */
export function umsetzen(r: Antwort): VorschauStatus {
  if (r.art === "bild") return { art: "bild", url: convertFileSrc(r.datei) };
  if (r.art === "datenbank") return { art: "bild", url: convertFileSrc(r.schluessel, "vorschau") };
  return r;
}

function weiter() {
  while (aktiv < MAX_PARALLEL && warteschlange.length) {
    // jüngste Anfrage zuerst: nach schnellem Scrollen kommt das Sichtbare vor dem Vorbeigerollten
    const a = warteschlange.pop()!;
    if (a.abgebrochen) continue;
    aktiv++;
    invoke<Antwort>("vorschau", { pfad: a.pfad, groesse: a.groesse })
      .then((r): VorschauStatus => umsetzen(r))
      .catch((e): VorschauStatus => ({ art: "fehler", meldung: String(e) }))
      .then((s) => {
        if (s.art === "bild" || s.art === "fehlt") ergebnisse.set(schluessel(a.pfad, a.groesse), s);
        a.fertig(s);
      })
      .finally(() => {
        aktiv--;
        weiter();
      });
  }
}

/** Auftrag einreihen; liefert eine Funktion zum Abbrechen (z. B. beim Wegscrollen). */
export function anfordern(pfad: string, groesse: number, fertig: (s: VorschauStatus) => void) {
  const a: Auftrag = { pfad, groesse, fertig, abgebrochen: false };
  warteschlange.push(a);
  weiter();
  return () => {
    a.abgebrochen = true;
  };
}

// Nach der Dropbox-Anmeldung sollen Wolken-Kacheln ihre Vorschau neu anfordern.
const hoerer = new Set<() => void>();

/** Alle angezeigten „nur online“-Vorschauen erneut anfordern (z. B. nach Dropbox-Anmeldung). */
export function nurOnlineNeuLaden() {
  hoerer.forEach((h) => h());
}

/** Alles neu anfordern – nach „Cache leeren“ zeigen gemerkte Adressen auf gelöschte Dateien. */
export function alleNeuLaden() {
  ergebnisse.clear();
  hoerer.forEach((h) => h());
}

/** Nur für Tests: Warteschlange, Zähler und Ergebnis-Cache leeren. */
export function _zuruecksetzen() {
  ergebnisse.clear();
  warteschlange.length = 0;
  aktiv = 0;
}

/** Vorschau eines Treffers (aus dem Cache oder neu erzeugt). */
export function useVorschau(pfad: string, groesse = VORSCHAU_GROESSE) {
  const key = schluessel(pfad, groesse);
  const [status, setStatus] = useState<VorschauStatus>(() => ergebnisse.get(key) ?? { art: "laedt" });
  const [runde, setRunde] = useState(0);

  // „nur online“ wird nicht gemerkt – bei neuer Runde fragt die Kachel erneut
  useEffect(() => {
    const h = () => setRunde((r) => r + 1);
    hoerer.add(h);
    return () => {
      hoerer.delete(h);
    };
  }, []);

  useEffect(() => {
    const vorhanden = ergebnisse.get(key);
    if (vorhanden) {
      setStatus(vorhanden);
      return;
    }
    setStatus({ art: "laedt" });
    return anfordern(pfad, groesse, setStatus);
  }, [key, pfad, groesse, runde]);

  return { status };
}
