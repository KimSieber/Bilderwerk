// Testdaten für Frontend-Tests
import type { DbInfo, Ergebnis, Facette, Treffer } from "../types";

export function treffer(id: number, t: Partial<Treffer> = {}): Treffer {
  return {
    id,
    pfad: `/Archiv/Reise/IMG_${id}.jpg`,
    relativerPfad: `Reise/IMG_${id}.jpg`,
    archivWurzel: "/Archiv",
    ordner: "Reise",
    dateiname: `IMG_${id}.jpg`,
    endung: ".jpg",
    kurztitel: `Bild ${id}`,
    aufnahmeDatum: "2015-06-09T14:12:36",
    seite: null,
    breite: 4000,
    hoehe: 3000,
    status: "ok",
    ...t,
  };
}

export function ergebnis(n: number, gesamt = n): Ergebnis {
  return { gesamt, treffer: Array.from({ length: n }, (_, i) => treffer(i + 1)), dauerMs: 7 };
}

export const info: DbInfo = {
  pfad: "/Users/kim/Dropbox/Bildkatalog.db",
  schemaVersion: 3,
  bilder: 23742,
  duplikate: 2500,
  geaendert: 1793000000,
  groesse: 1000,
  vorschauen: null,
};

export const facetten: Facette[] = [
  { feld: "motiv", titel: "Motiv", werte: [{ wert: "Landschaft", anzahl: 12 }, { wert: "Porträt", anzahl: 3 }] },
  { feld: "merkmale", titel: "Merkmale", werte: [{ wert: "brille", anzahl: 2 }] },
];
