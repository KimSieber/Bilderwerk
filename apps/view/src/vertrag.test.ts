// V: Vertrag Rust ↔ Frontend. Die JSON-Dateien erzeugt `cargo test` (src-tauri/tests/vertrag.rs)
// aus den echten Rust-Strukturen. Weicht ein Feld ab, schlagen diese Tests an.
import { fireEvent, render, screen } from "@testing-library/react";
import { createElement } from "react";
import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (p: string, proto = "asset") => `${proto}://${p}`,
}));
vi.mock("react-virtuoso", () => import("./test/virtuosoMock"));
vi.mock("./components/Vorschaubild", () => ({ default: () => null }));
import { invoke } from "@tauri-apps/api/core";
import ergebnisJson from "../tests/vertrag/ergebnis.json";
import ergebnisLeerJson from "../tests/vertrag/ergebnis-leer.json";
import dbinfoJson from "../tests/vertrag/dbinfo.json";
import vBild from "../tests/vertrag/vorschau-bild.json";
import vNurOnline from "../tests/vertrag/vorschau-nuronline.json";
import vFehlt from "../tests/vertrag/vorschau-fehlt.json";
import vFehler from "../tests/vertrag/vorschau-fehler.json";
import dropboxJson from "../tests/vertrag/dropbox-status.json";
import cacheJson from "../tests/vertrag/cache-info.json";
import vDatenbank from "../tests/vertrag/vorschau-datenbank.json";
import facettenJson from "../tests/vertrag/facetten.json";
import detailsJson from "../tests/vertrag/details.json";
import einstellungenJson from "../tests/vertrag/einstellungen.json";
import { aufbereiten } from "./detailfelder";
import type {
  CacheInfo,
  DbInfo,
  Details,
  DetailFeld,
  DropboxStatus,
  Einstellungen,
  Verwandt,
  Ergebnis,
  Facette,
  FacettenWert,
  Regel,
  Treffer,
} from "./types";
import { ANBIETER_ARTEN } from "./types";
import Facetten from "./components/Facetten";
import { _zuruecksetzen, anfordern } from "./vorschau";
import Results from "./components/Results";

// Feldlisten der TypeScript-Typen – Record<keyof …> erzwingt Vollständigkeit beim Typcheck
const TREFFER: Record<keyof Treffer, true> = {
  id: true, pfad: true, relativerPfad: true, archivWurzel: true, ordner: true, dateiname: true, endung: true,
  kurztitel: true, aufnahmeDatum: true, seite: true, breite: true, hoehe: true, status: true,
};
const ERGEBNIS: Record<keyof Ergebnis, true> = { gesamt: true, treffer: true, dauerMs: true };
const DBINFO: Record<keyof DbInfo, true> = {
  pfad: true, schemaVersion: true, bilder: true, duplikate: true, geaendert: true, groesse: true, vorschauen: true,
};
const CACHE: Record<keyof CacheInfo, true> = { bytes: true, dateien: true };
const DROPBOX: Record<keyof DropboxStatus, true> = { eingerichtet: true, angemeldet: true };
const FACETTE: Record<keyof Facette, true> = { feld: true, titel: true, werte: true };
const FACETTENWERT: Record<keyof FacettenWert, true> = { wert: true, anzahl: true };
const DETAILS: Record<keyof Details, true> = { id: true, pfad: true, felder: true, begriffe: true, duplikate: true };
const DETAILFELD: Record<keyof DetailFeld, true> = { name: true, wert: true };
const VERWANDT: Record<keyof Verwandt, true> = { id: true, pfad: true, status: true };
const EINSTELLUNGEN: Record<keyof Einstellungen, true> = {
  dbPfad: true, cacheGrenzeMb: true, anbieter: true, dropboxAppKey: true, standardAppKey: true,
};
const REGEL: Record<keyof Regel, true> = { pfad: true, art: true };
const felder = (o: object) => Object.keys(o).sort();

describe("Vertrag", () => {
  it("Ergebnis und Treffer haben genau die Felder der TS-Typen (V-05)", () => {
    expect(felder(ergebnisJson)).toEqual(felder(ERGEBNIS));
    expect(felder(ergebnisJson.treffer[0])).toEqual(felder(TREFFER));
    expect(ergebnisLeerJson).toEqual({ gesamt: 0, treffer: [], dauerMs: 0 });
  });

  it("DbInfo hat genau die Felder des TS-Typs (V-06)", () => {
    expect(felder(dbinfoJson)).toEqual(felder(DBINFO));
  });

  it("echte Rust-Antwort lässt sich anzeigen (V-07)", () => {
    render(createElement(Results, { ergebnis: ergebnisJson as Ergebnis, auswahl: null, onAuswahl: () => {} }));
    expect(screen.getByText(/2 Bilder/)).toBeInTheDocument();
    expect(screen.getByText("Albumseite mit Fotos")).toBeInTheDocument();
    expect(screen.getByText("01.08.2008 · S. 1")).toBeInTheDocument();
  });

  it("alle Vorschau-Antworten werden verstanden (V-08)", async () => {
    _zuruecksetzen();
    const antworten = [vBild, vNurOnline, vFehlt, vFehler, vDatenbank];
    let i = 0;
    vi.mocked(invoke).mockImplementation(async () => antworten[i++]);
    const s: unknown[] = [];
    for (let n = 0; n < antworten.length; n++) anfordern(`/v${n}.jpg`, 600, (x) => s.push(x));
    await new Promise((r) => setTimeout(r, 0));
    expect(s).toContainEqual({ art: "bild", url: `asset://${vBild.datei}` });
    expect(s).toContainEqual({ art: "nurOnline" });
    expect(s).toContainEqual({ art: "fehlt" });
    expect(s).toContainEqual({ art: "fehler", meldung: vFehler.meldung });
    expect(s).toContainEqual({ art: "bild", url: `vorschau://${vDatenbank.schluessel}` });
  });

  it("DropboxStatus hat genau die Felder des TS-Typs (V-10)", () => {
    expect(felder(dropboxJson)).toEqual(felder(DROPBOX));
  });

  it("CacheInfo hat genau die Felder des TS-Typs (V-12)", () => {
    expect(felder(cacheJson)).toEqual(felder(CACHE));
  });

  it("Facetten haben genau die Felder der TS-Typen und lassen sich anzeigen (V-14)", async () => {
    expect(felder(facettenJson[0])).toEqual(felder(FACETTE));
    expect(felder(facettenJson[0].werte[0])).toEqual(felder(FACETTENWERT));
    render(createElement(Facetten, { facetten: facettenJson as Facette[], auswahl: { motiv: ["Porträt"] }, onChange: () => {} }));
    fireEvent.click(screen.getByRole("button", { name: /^Motiv/ }));
    expect(screen.getByRole("button", { name: /^Menschen in Aktion/ })).toHaveTextContent("19");
  });

  it("Details: Felder, Begriffe, Duplikate wie die TS-Typen; Aufbereitung klappt (V-16)", () => {
    expect(felder(detailsJson)).toEqual(felder(DETAILS));
    expect(felder(detailsJson.felder[0])).toEqual(felder(DETAILFELD));
    expect(felder(detailsJson.duplikate[0])).toEqual(felder(VERWANDT));
    const a = aufbereiten(detailsJson as Details);
    expect(a.titel).toBe("Strand mit Booten");
    expect(a.chips[0]).toEqual({ titel: "Schlagworte", werte: ["boot", "strand", "meer"] });
    expect(a.gruppen.find((g) => g.titel === "Bildinhalt")?.zeilen[0]).toEqual({ label: "Motiv", wert: "Landschaft" });
  });

  it("Einstellungen und Regel haben genau die Felder der TS-Typen; Arten bekannt (V-18)", () => {
    expect(felder(einstellungenJson)).toEqual(felder(EINSTELLUNGEN));
    expect(felder(einstellungenJson.anbieter[0])).toEqual(felder(REGEL));
    const arten = ANBIETER_ARTEN.map((a) => a.art);
    for (const r of einstellungenJson.anbieter) expect(arten).toContain(r.art);
  });
});
