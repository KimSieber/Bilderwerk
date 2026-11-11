import { describe, expect, it } from "vitest";
// @ts-expect-error – reines JS-Skript ohne Typen
import { ohneFarbe, rustSumme, vitestSumme } from "./tests.mjs";

describe("Testsumme (F-LOG-24)", () => {
  it("summiert alle cargo-Blöcke", () => {
    const t = `test result: ok. 69 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 5 passed; 0 failed; 1 ignored; 0 measured
test result: FAILED. 12 passed; 1 failed; 0 ignored; 0 measured`;
    expect(rustSumme(t)).toEqual({ bestanden: 86, fehlgeschlagen: 1, uebersprungen: 1 });
    expect(rustSumme("")).toEqual({ bestanden: 0, fehlgeschlagen: 0, uebersprungen: 0 });
  });
  it("liest die Vitest-Zeile (grün und rot)", () => {
    expect(vitestSumme(" Test Files  12 passed (12)\n      Tests  78 passed (78)\n")).toEqual({ bestanden: 78, fehlgeschlagen: 0, uebersprungen: 0 });
    expect(vitestSumme("      Tests  2 failed | 76 passed (78)")).toEqual({ bestanden: 76, fehlgeschlagen: 2, uebersprungen: 0 });
  });
  it("versteht auch eingefärbte Ausgabe (cargo mit Farbe)", () => {
    const t = "test result: \x1b[32mok\x1b(B\x1b[m. 69 passed; 0 failed; 0 ignored; 0 measured";
    expect(rustSumme(ohneFarbe(t))).toEqual({ bestanden: 69, fehlgeschlagen: 0, uebersprungen: 0 });
    expect(ohneFarbe("\x1b[1m\x1b[32mTests\x1b[39m\x1b[22m  5 passed")).toBe("Tests  5 passed");
  });
});
