// Detailbereich (v0.0.6): ordnet die Spalten aus `bilder` zu lesbaren Gruppen.
// Unbekannte Spalten (neue Work-Felder) erscheinen unter „Weitere“ – nichts geht verloren.
import { datumDe, groesseDe, zahlDe } from "./api";
import type { Details } from "./types";

export interface Zeile {
  label: string;
  wert: string;
}
export interface Gruppe {
  titel: string;
  zeilen: Zeile[];
  /** anfangs zugeklappt (technische Angaben) */
  zu?: boolean;
  /** längerer Text ohne Beschriftung */
  block?: boolean;
}
export interface Aufbereitet {
  titel: string;
  beschreibung: string | null;
  gruppen: Gruppe[];
  chips: { titel: string; werte: string[] }[];
}

const zahl = (n: number, stellen = 2) => n.toLocaleString("de-DE", { maximumFractionDigits: stellen });

/** "2019-07-14T21:42:10" → "14.07.2019 21:42"; unplausibel → roh (wie datumDe) */
export function datumZeitDe(iso: string): string {
  const d = datumDe(iso);
  if (!/^\d\d\.\d\d\.\d{4}$/.test(d)) return d;
  const z = /T(\d\d):(\d\d)/.exec(iso);
  return z ? `${d} ${z[1]}:${z[2]}` : d;
}

/** Unix-Sekunden → "14.07.2019 21:42" in Ortszeit */
export function lokalDe(sek: number): string {
  const t = new Date(sek * 1000);
  const z = (n: number) => String(n).padStart(2, "0");
  return `${z(t.getDate())}.${z(t.getMonth() + 1)}.${t.getFullYear()} ${z(t.getHours())}:${z(t.getMinutes())}`;
}

/** 0,004 → "1/250 s"; ≥ 1 s → "2 s" */
export function belichtungDe(s: number): string {
  if (s >= 1) return `${zahl(s, 1)} s`;
  return `1/${Math.round(1 / s)} s`;
}

export function gpsDe(lat: number, lon: number): string {
  const f = (v: number) => zahl(Math.abs(v), 4);
  return `${f(lat)}° ${lat < 0 ? "S" : "N"}, ${f(lon)}° ${lon < 0 ? "W" : "O"}`;
}

/** JSON-Liste oder Komma/Semikolon-Text → Werte */
export function liste(v: string | number | undefined): string[] {
  if (v === undefined || v === "") return [];
  const s = String(v).trim();
  if (s.startsWith("[")) {
    try {
      const a = JSON.parse(s);
      if (Array.isArray(a)) return a.map((x) => (typeof x === "string" ? x : JSON.stringify(x))).filter(Boolean);
    } catch {
      return [s];
    }
  }
  return s.split(/[,;]/).map((x) => x.trim()).filter(Boolean);
}

type Werte = Record<string, string | number>;
type Regel = [label: string, spalten: string[], format?: (w: Werte) => string | null];

const text = (n: string) => (w: Werte) => (w[n] === undefined ? null : String(w[n]));
const num = (n: string) => (w: Werte) => (typeof w[n] === "number" ? zahl(w[n] as number) : text(n)(w));
const listeText = (n: string) => (w: Werte) => liste(w[n]).join(", ") || null;

const QUELLE: Record<string, string> = { dateiname: "aus Dateiname", metadaten: "" };

const AUFNAHME: Regel[] = [
  ["Datum", ["aufnahme_datum", "datum_quelle"], (w) => {
    if (w.aufnahme_datum === undefined) return null;
    const q = w.datum_quelle !== undefined ? QUELLE[String(w.datum_quelle)] ?? String(w.datum_quelle) : "";
    return datumZeitDe(String(w.aufnahme_datum)) + (q ? ` (${q})` : "");
  }],
  ["Ort", ["ort"]],
  ["GPS", ["gps_lat", "gps_lon", "gps_hoehe"], (w) => {
    if (typeof w.gps_lat !== "number" || typeof w.gps_lon !== "number") return null;
    return gpsDe(w.gps_lat, w.gps_lon) + (typeof w.gps_hoehe === "number" ? ` · ${Math.round(w.gps_hoehe)} m` : "");
  }],
  ["Kamera", ["kamera_hersteller", "kamera_modell"], (w) => {
    const h = w.kamera_hersteller !== undefined ? String(w.kamera_hersteller) : "";
    const m = w.kamera_modell !== undefined ? String(w.kamera_modell) : "";
    if (!h && !m) return null;
    return m.toLowerCase().startsWith(h.toLowerCase()) ? m || h : `${h} ${m}`.trim();
  }],
  ["Objektiv", ["objektiv"]],
  ["Brennweite", ["brennweite", "brennweite_kb"], (w) => {
    if (typeof w.brennweite !== "number") return typeof w.brennweite_kb === "number" ? `KB ${zahl(w.brennweite_kb)} mm` : null;
    return `${zahl(w.brennweite)} mm` + (typeof w.brennweite_kb === "number" ? ` (KB ${zahl(w.brennweite_kb)} mm)` : "");
  }],
  ["Blende", ["blende"], (w) => (typeof w.blende === "number" ? `f/${zahl(w.blende, 1)}` : text("blende")(w))],
  ["Belichtung", ["belichtungszeit"], (w) => (typeof w.belichtungszeit === "number" && w.belichtungszeit > 0 ? belichtungDe(w.belichtungszeit) : text("belichtungszeit")(w))],
  ["ISO", ["iso"], num("iso")],
  ["Blitz", ["blitz"]],
];

const DATEI: Regel[] = [
  ["Ablage", ["relativer_pfad"], (w) => text("relativer_pfad")(w) ?? text("pfad")(w)],
  ["Archiv", ["archiv_wurzel"]],
  ["Format", ["endung", "breite", "hoehe"], (w) => {
    const typ = w.endung !== undefined ? String(w.endung).replace(/^\./, "").toUpperCase() : "";
    const m = typeof w.breite === "number" && typeof w.hoehe === "number" ? `${zahlDe(w.breite)} × ${zahlDe(w.hoehe)}` : "";
    return [typ, m].filter(Boolean).join(" · ") || null;
  }],
  ["Größe", ["groesse"], (w) => (typeof w.groesse === "number" ? groesseDe(w.groesse) : null)],
  ["Seite", ["seite", "seiten_gesamt"], (w) => (w.seite === undefined ? null : `${w.seite}${w.seiten_gesamt !== undefined ? ` / ${w.seiten_gesamt}` : ""}`)],
  ["Geändert", ["mtime"], (w) => (typeof w.mtime === "number" ? lokalDe(w.mtime) : null)],
];

const INHALT: Regel[] = [
  ["Motiv", ["motiv"]],
  ["Bildart", ["bildart"]],
  ["Anlass", ["anlass"]],
  ["Szene", ["szene"]],
  ["Innen/außen", ["innen_aussen"]],
  ["Tageszeit", ["tageszeit"]],
  ["Jahreszeit", ["jahreszeit"]],
  ["Wetter", ["wetter"]],
  ["Farbgebung", ["farbgebung"]],
  ["Ausrichtung", ["ausrichtung"]],
  ["Personen", ["personen_anzahl"], num("personen_anzahl")],
  ["Stimmung", ["stimmung"], listeText("stimmung")],
];

const META: Regel[] = [
  ["Titel", ["meta_titel"]],
  ["Beschreibung", ["meta_beschreibung"]],
  ["Urheber", ["urheber"]],
  ["Copyright", ["copyright"]],
  ["Software", ["software"]],
];

const ANALYSE: Regel[] = [
  ["Status", ["status"]],
  ["Fehler", ["fehler"]],
  ["Modell", ["modell"]],
  ["Prompt-Version", ["prompt_version"]],
  ["Tokens", ["tokens"], num("tokens")],
  ["Dauer", ["dauer_s"], (w) => (typeof w.dauer_s === "number" ? `${zahl(w.dauer_s, 1)} s` : null)],
  ["Erfasst", ["erfasst"], (w) => (w.erfasst === undefined ? null : datumZeitDe(String(w.erfasst)))],
  ["Motiv (roh)", ["motiv_roh"]],
  ["Einzelbilder", ["einzelbilder"], listeText("einzelbilder")],
  ["Hash", ["hash"]],
  ["ID", ["id"]],
];

/** Spalten, die woanders erscheinen (Kopf, Chips, Duplikate) oder redundant sind */
const VERBRAUCHT = ["pfad", "ordner", "dateiname", "kurztitel", "beschreibung", "schlagworte", "personen_merkmale",
  "meta_stichworte", "duplikat_von", "sichtbarer_text", "metadaten_json"];

export function aufbereiten(d: Details): Aufbereitet {
  const w: Werte = Object.fromEntries(d.felder.map((f) => [f.name, f.wert]));
  const benutzt = new Set(VERBRAUCHT);

  const gruppe = (titel: string, regeln: Regel[], zu = false): Gruppe => ({
    titel,
    zu,
    zeilen: regeln.flatMap(([label, spalten, format]) => {
      spalten.forEach((s) => benutzt.add(s));
      const wert = (format ?? text(spalten[0]))(w);
      return wert ? [{ label, wert }] : [];
    }),
  });

  const gruppen = [
    gruppe("Aufnahme", AUFNAHME),
    gruppe("Bildinhalt", INHALT),
    gruppe("Datei", DATEI),
  ];
  if (w.sichtbarer_text !== undefined)
    gruppen.push({ titel: "Sichtbarer Text", block: true, zeilen: [{ label: "", wert: String(w.sichtbarer_text) }] });
  gruppen.push(gruppe("Metadaten der Datei", META), gruppe("Analyse", ANALYSE, true));

  if (w.metadaten_json !== undefined) {
    let zeilen: Zeile[];
    try {
      const o = JSON.parse(String(w.metadaten_json));
      zeilen = Object.entries(o ?? {}).map(([k, v]) => ({ label: k, wert: typeof v === "string" ? v : JSON.stringify(v) }));
    } catch {
      zeilen = [{ label: "metadaten_json", wert: String(w.metadaten_json) }];
    }
    gruppen.push({ titel: "Alle Metadaten", zu: true, zeilen });
  }
  const rest = d.felder.filter((f) => !benutzt.has(f.name)).map((f) => ({ label: f.name, wert: String(f.wert) }));
  gruppen.push({ titel: "Weitere", zu: true, zeilen: rest });

  const begriff = (typ: string, spalte: string) => (d.begriffe[typ]?.length ? d.begriffe[typ] : liste(w[spalte]));
  const chips = [
    { titel: "Schlagworte", werte: begriff("schlagwort", "schlagworte") },
    { titel: "Merkmale", werte: begriff("merkmal", "personen_merkmale") },
    { titel: "Stichworte", werte: begriff("stichwort", "meta_stichworte") },
  ].filter((c) => c.werte.length > 0);

  const name = d.pfad.replace(/#seite=\d+$/, "").split("/").pop() || d.pfad;
  return {
    titel: w.kurztitel !== undefined ? String(w.kurztitel) : name,
    beschreibung: w.beschreibung !== undefined ? String(w.beschreibung) : null,
    gruppen: gruppen.filter((g) => g.zeilen.length > 0),
    chips,
  };
}
