import { describe, expect, it } from "vitest";
import { begrenzen, DETAIL_MIN, ergebnisMin, KOPF_MIN, LISTE_MIN, TEILER } from "./teiler";

describe("Detailbreite (F-LOG-29)", () => {
  it("Ergebnis braucht eine Kachel bzw. die Listen-Mindestbreite", () => {
    expect(ergebnisMin("raster", 110)).toBe(KOPF_MIN); // Kopfzeile passt immer
    expect(ergebnisMin("raster", 240)).toBe(240 + 12 + 16);
    expect(ergebnisMin("raster", 340)).toBe(340 + 12 + 16);
    expect(ergebnisMin("liste", 180)).toBe(LISTE_MIN);
  });
  it("begrenzt nach unten und oben", () => {
    expect(begrenzen(100, 1300, 208)).toBe(DETAIL_MIN);
    expect(begrenzen(500, 1300, 208)).toBe(500);
    expect(begrenzen(5000, 1300, 208)).toBe(1300 - TEILER - 208);
    expect(begrenzen(5000, 1300, LISTE_MIN)).toBe(1300 - TEILER - LISTE_MIN);
  });
  it("schmales Fenster: Mindestbreite gewinnt", () => {
    expect(begrenzen(400, 500, 420)).toBe(DETAIL_MIN);
  });
});
