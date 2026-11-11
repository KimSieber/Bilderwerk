// F-UI-50: Kacheln und Liste zeigen das ganze Bild (Hoch-/Querformat erkennbar), kein Zuschnitt.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

// Vitest läuft im Ordner app/ (dort liegt package.json)
const css = readFileSync("src/styles.css", "utf8");
const regel = (sel: string) => {
  const i = css.indexOf(`${sel} {`);
  expect(i, sel).toBeGreaterThanOrEqual(0);
  return css.slice(i, css.indexOf("}", i));
};

describe("Vorschau-Darstellung (F-UI-50)", () => {
  it("ganzes Bild statt Zuschnitt", () => {
    expect(regel(".vorschau__bild")).toMatch(/object-fit:\s*contain/);
    expect(css).not.toMatch(/object-fit:\s*cover/);
  });
  it("ruhiger Bildgrund in Kachel und Liste, Kachel bleibt 4:3", () => {
    expect(regel(".kachel__bild")).toMatch(/aspect-ratio:\s*4 \/ 3/);
    expect(regel(".kachel__bild")).toMatch(/background:\s*var\(--bildgrund\)/);
    expect(regel(".zeile__vorschau")).toMatch(/background:\s*var\(--bildgrund\)/);
    expect(css.match(/--bildgrund:/g)?.length).toBe(2); // hell und dunkel
  });
});

describe("Detailbereich (F-UI-68)", () => {
  it("Vorschau ganz, ruhiger Grund, fester Platz rechts", () => {
    expect(regel(".detail__bild .vorschau__bild")).toMatch(/object-fit:\s*contain/);
    expect(regel(".detail__bild")).toMatch(/background:\s*var\(--bildgrund\)/);
    expect(regel(".detail")).toMatch(/flex:\s*0 0 var\(--detailbreite/);
    expect(regel(".detail")).toMatch(/overflow-y:\s*auto/);
    expect(regel(".teiler")).toMatch(/cursor:\s*col-resize/);
  });
});

