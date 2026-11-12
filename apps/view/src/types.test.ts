import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { DATEITYPEN, LEERER_FILTER } from "./types";
import { istLeer } from "./App";

describe("Dateitypen (F-LOG-04)", () => {
  const alle = DATEITYPEN.flatMap((t) => t.endungen);
  it("decken alle Endungen des Katalogs ab", () => {
    for (const e of [".jpg", ".jpeg", ".png", ".heic", ".heif", ".tif", ".tiff", ".webp", ".pdf"]) {
      expect(alle).toContain(e);
    }
    // RAW wie Work (shared/bilderwerk/bild.py RAW_ENDUNGEN), v0.3.2
    for (const e of [".cr2", ".cr3", ".nef", ".nrw", ".arw", ".dng", ".orf", ".raf", ".rw2", ".pef", ".srw"]) {
      expect(alle).toContain(e);
    }
  });
  it("RAW-Gruppe entspricht genau RAW_ENDUNGEN in shared/bilderwerk/bild.py (F-LOG-32)", () => {
    const py = readFileSync("../../shared/bilderwerk/bild.py", "utf8");
    const block = /RAW_ENDUNGEN = \{([^}]*)\}/.exec(py)![1];
    const work = [...block.matchAll(/"(\.[a-z0-9]+)"/g)].map((m) => m[1]).sort();
    const view = DATEITYPEN.find((t) => t.label === "RAW")!.endungen.slice().sort();
    expect(view).toEqual(work);
  });
  it("keine Endung doppelt, alle klein mit Punkt", () => {
    expect(new Set(alle).size).toBe(alle.length);
    for (const e of alle) expect(e).toMatch(/^\.[a-z0-9]+$/);
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
