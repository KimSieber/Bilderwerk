import { describe, expect, it } from "vitest";
import { aufbereiten, belichtungDe, datumZeitDe, gpsDe, liste, lokalDe } from "./detailfelder";
import type { Details } from "./types";

function d(felder: Record<string, string | number>, begriffe: Record<string, string[]> = {}): Details {
  return {
    id: 1,
    pfad: String(felder.pfad ?? "/A/x.jpg"),
    felder: Object.entries(felder).map(([name, wert]) => ({ name, wert })),
    begriffe,
    duplikate: [],
  };
}
const gruppe = (a: ReturnType<typeof aufbereiten>, titel: string) => a.gruppen.find((g) => g.titel === titel);
const zeile = (a: ReturnType<typeof aufbereiten>, titel: string, label: string) =>
  gruppe(a, titel)?.zeilen.find((z) => z.label === label)?.wert;

describe("Formate Detailbereich (F-LOG-27)", () => {
  it("Datum mit Uhrzeit, unplausibel roh", () => {
    expect(datumZeitDe("2019-07-14T21:42:10")).toBe("14.07.2019 21:42");
    expect(datumZeitDe("2019-07-14")).toBe("14.07.2019");
    expect(datumZeitDe("2056-55-10T00:00:00")).toBe("2056-55-10");
  });
  it("Änderungszeit in Ortszeit", () => {
    const t = new Date(2020, 0, 5, 7, 3).getTime() / 1000;
    expect(lokalDe(t)).toBe("05.01.2020 07:03");
  });
  it("Belichtungszeit als Bruch", () => {
    expect(belichtungDe(0.004)).toBe("1/250 s");
    expect(belichtungDe(0.5)).toBe("1/2 s");
    expect(belichtungDe(2)).toBe("2 s");
    expect(belichtungDe(1.5)).toBe("1,5 s");
  });
  it("GPS mit Himmelsrichtung", () => {
    expect(gpsDe(48.77585, 9.18293)).toBe("48,7759° N, 9,1829° O");
    expect(gpsDe(-33.9, -70.1)).toBe("33,9° S, 70,1° W");
  });
  it("JSON-Listen zerlegen, sonst Komma-Text", () => {
    expect(liste('["a","b"]')).toEqual(["a", "b"]);
    expect(liste("a, b;c")).toEqual(["a", "b", "c"]);
    expect(liste(undefined)).toEqual([]);
    expect(liste("[kaputt")).toEqual(["[kaputt"]);
  });
});

describe("aufbereiten (F-LOG-28)", () => {
  const voll = d(
    {
      id: 1, pfad: "/A/Reise/IMG_1.heic", archiv_wurzel: "/A", relativer_pfad: "Reise/IMG_1.heic", ordner: "Reise",
      dateiname: "IMG_1.heic", endung: ".heic", groesse: 3_250_000, breite: 4032, hoehe: 3024,
      aufnahme_datum: "2019-07-14T21:42:10", datum_quelle: "dateiname", kamera_hersteller: "Apple", kamera_modell: "iPhone XS",
      brennweite: 4.25, brennweite_kb: 26, blende: 1.8, belichtungszeit: 0.004, iso: 100, gps_lat: 54.9, gps_lon: 8.3,
      kurztitel: "Sonnenuntergang", beschreibung: "Abendlicht am Meer.", motiv: "Landschaft", jahreszeit: "Sommer",
      stimmung: '["ruhig","warm"]', schlagworte: '["strand","meer"]', personen_merkmale: '["brille"]',
      sichtbarer_text: "STOP", meta_titel: "Urlaub", status: "ok", modell: "qwen", hash: "abc",
      metadaten_json: '{"EXIF:Make":"Apple","EXIF:ISO":100}', neue_spalte: "neu",
    },
    { schlagwort: ["strand", "meer", "sonne"] },
  );
  const a = aufbereiten(voll);

  it("Kopf: Kurztitel und Beschreibung", () => {
    expect(a.titel).toBe("Sonnenuntergang");
    expect(a.beschreibung).toBe("Abendlicht am Meer.");
  });
  it("Aufnahme: Datum mit Quelle, Kamera zusammengefasst, Technik formatiert", () => {
    expect(zeile(a, "Aufnahme", "Datum")).toBe("14.07.2019 21:42 (aus Dateiname)");
    expect(zeile(a, "Aufnahme", "Kamera")).toBe("Apple iPhone XS");
    expect(zeile(a, "Aufnahme", "Brennweite")).toBe("4,25 mm (KB 26 mm)");
    expect(zeile(a, "Aufnahme", "Blende")).toBe("f/1,8");
    expect(zeile(a, "Aufnahme", "Belichtung")).toBe("1/250 s");
    expect(zeile(a, "Aufnahme", "ISO")).toBe("100");
    expect(zeile(a, "Aufnahme", "GPS")).toBe("54,9° N, 8,3° O");
  });
  it("Datei: Ablage, Format mit Auflösung, Größe", () => {
    expect(zeile(a, "Datei", "Ablage")).toBe("Reise/IMG_1.heic");
    expect(zeile(a, "Datei", "Format")).toBe("HEIC · 4.032 × 3.024");
    expect(zeile(a, "Datei", "Größe")).toBe("3 MB");
  });
  it("Bildinhalt mit Listen", () => {
    expect(zeile(a, "Bildinhalt", "Motiv")).toBe("Landschaft");
    expect(zeile(a, "Bildinhalt", "Stimmung")).toBe("ruhig, warm");
  });
  it("Chips: Begriffe-Tabelle vor Spalte, Spalte als Rückfall", () => {
    expect(a.chips).toEqual([
      { titel: "Schlagworte", werte: ["strand", "meer", "sonne"] },
      { titel: "Merkmale", werte: ["brille"] },
    ]);
  });
  it("Text, Metadaten der Datei, Analyse und alle Metadaten (zugeklappt)", () => {
    expect(zeile(a, "Sichtbarer Text", "")).toBe("STOP");
    expect(zeile(a, "Metadaten der Datei", "Titel")).toBe("Urlaub");
    expect(gruppe(a, "Analyse")?.zu).toBe(true);
    expect(zeile(a, "Analyse", "Modell")).toBe("qwen");
    expect(gruppe(a, "Alle Metadaten")?.zu).toBe(true);
    expect(zeile(a, "Alle Metadaten", "EXIF:Make")).toBe("Apple");
    expect(zeile(a, "Alle Metadaten", "EXIF:ISO")).toBe("100");
  });
  it("unbekannte Spalten unter „Weitere“, nichts doppelt", () => {
    expect(zeile(a, "Weitere", "neue_spalte")).toBe("neu");
    const labels = a.gruppen.flatMap((g) => g.zeilen.map((z) => `${g.titel}/${z.label}`));
    expect(new Set(labels).size).toBe(labels.length);
    expect(labels.some((l) => /ordner|dateiname|kurztitel|schlagworte|personen_merkmale/.test(l))).toBe(false);
  });
  it("leere Gruppen entfallen; PDF-Seite; Kamera ohne doppelten Hersteller", () => {
    const b = aufbereiten(d({ pfad: "/A/x.pdf#seite=2", seite: 2, seiten_gesamt: 12, kamera_hersteller: "Canon", kamera_modell: "Canon EOS 5D" }));
    expect(b.titel).toBe("x.pdf");
    expect(zeile(b, "Datei", "Seite")).toBe("2 / 12");
    expect(zeile(b, "Aufnahme", "Kamera")).toBe("Canon EOS 5D");
    expect(gruppe(b, "Bildinhalt")).toBeUndefined();
    expect(b.chips).toEqual([]);
  });
  it("kaputtes metadaten_json bleibt als Text sichtbar", () => {
    const c = aufbereiten(d({ metadaten_json: "{kaputt" }));
    expect(zeile(c, "Alle Metadaten", "metadaten_json")).toBe("{kaputt");
  });
});
