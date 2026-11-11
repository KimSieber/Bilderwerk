import { describe, expect, it } from "vitest";
import { DATEITYPEN, LEERER_FILTER } from "./types";
import { istLeer } from "./App";

describe("Dateitypen (F-LOG-04)", () => {
  const alle = DATEITYPEN.flatMap((t) => t.endungen);
  it("decken alle Endungen des Katalogs ab", () => {
    for (const e of [".jpg", ".jpeg", ".png", ".heic", ".heif", ".tif", ".tiff", ".webp", ".pdf"]) {
      expect(alle).toContain(e);
    }
  });
  it("keine Endung doppelt, alle klein mit Punkt", () => {
    expect(new Set(alle).size).toBe(alle.length);
    for (const e of alle) expect(e).toMatch(/^\.[a-z]+$/);
  });
});

describe("istLeer (F-LOG-05)", () => {
  it("leerer Filter und nur Leerzeichen/Duplikate gelten als leer", () => {
    expect(istLeer(LEERER_FILTER)).toBe(true);
    expect(istLeer({ ...LEERER_FILTER, freitext: "  ", mitDuplikaten: true })).toBe(true);
  });
  it("jedes Kriterium macht den Filter nicht leer", () => {
    for (const k of ["freitext", "name", "ordner", "datumVon", "datumBis"] as const) {
      expect(istLeer({ ...LEERER_FILTER, [k]: "x" })).toBe(false);
    }
    expect(istLeer({ ...LEERER_FILTER, endungen: [".pdf"] })).toBe(false);
  });
});

describe("istLeer mit Facetten (F-LOG-25)", () => {
  it("gewählte Facette zählt als Kriterium, leere Auswahl nicht", () => {
    expect(istLeer({ ...LEERER_FILTER, facetten: { motiv: ["Tier"] } })).toBe(false);
    expect(istLeer({ ...LEERER_FILTER, facetten: { motiv: [] } })).toBe(true);
    expect(LEERER_FILTER.facetten).toEqual({});
  });
});
