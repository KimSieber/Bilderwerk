import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (p: string, proto = "asset") =>
    proto === "asset" ? `asset://localhost${p}` : `${proto}://localhost/${encodeURIComponent(p)}`,
}));
import { invoke } from "@tauri-apps/api/core";
import {
  MAX_PARALLEL,
  VORSCHAU_GROESSE,
  _zuruecksetzen,
  alleNeuLaden,
  anfordern,
  nurOnlineNeuLaden,
  umsetzen,
  useVorschau,
} from "./vorschau";

type Antwort = { art: string; datei?: string; meldung?: string; schluessel?: string };
const inv = vi.mocked(invoke);

/** invoke liefert offene Versprechen; der Test löst sie gezielt auf. */
let offen: { pfad: string; loese: (a: Antwort) => void; lehneAb: (e: unknown) => void }[];
beforeEach(() => {
  _zuruecksetzen();
  offen = [];
  inv.mockReset().mockImplementation((_cmd, args) => {
    const pfad = (args as { pfad: string }).pfad;
    return new Promise((loese, lehneAb) => offen.push({ pfad, loese: loese as (a: Antwort) => void, lehneAb }));
  });
});
const tick = () => new Promise((r) => setTimeout(r, 0));

describe("Warteschlange (F-LOG-06…10)", () => {
  it(`höchstens ${MAX_PARALLEL} gleichzeitig (F-LOG-06)`, async () => {
    for (let i = 0; i < 10; i++) anfordern(`/b${i}.jpg`, 600, () => {});
    expect(inv).toHaveBeenCalledTimes(MAX_PARALLEL);
    offen[0].loese({ art: "fehlt" });
    await tick();
    expect(inv).toHaveBeenCalledTimes(MAX_PARALLEL + 1);
  });

  it("jüngste Anfrage zuerst – LIFO (F-LOG-07)", async () => {
    for (let i = 0; i < MAX_PARALLEL; i++) anfordern(`/voll${i}.jpg`, 600, () => {});
    anfordern("/alt.jpg", 600, () => {});
    anfordern("/neu.jpg", 600, () => {});
    offen[0].loese({ art: "fehlt" });
    await tick();
    expect(offen[MAX_PARALLEL].pfad).toBe("/neu.jpg");
  });

  it("abgebrochene Aufträge werden übersprungen (F-LOG-08)", async () => {
    for (let i = 0; i < MAX_PARALLEL; i++) anfordern(`/voll${i}.jpg`, 600, () => {});
    const fertig = vi.fn();
    const abbrechen = anfordern("/weg.jpg", 600, fertig);
    abbrechen();
    offen[0].loese({ art: "fehlt" });
    await tick();
    expect(inv).toHaveBeenCalledTimes(MAX_PARALLEL);
    expect(fertig).not.toHaveBeenCalled();
  });

  it("Antworten werden umgesetzt, Fehler abgefangen (F-LOG-09)", async () => {
    const s: unknown[] = [];
    anfordern("/a.jpg", 600, (x) => s.push(x));
    anfordern("/b.jpg", 600, (x) => s.push(x));
    anfordern("/c.jpg", 600, (x) => s.push(x));
    offen[0].loese({ art: "bild", datei: "/cache/1.jpg" });
    offen[1].loese({ art: "nurOnline" });
    offen[2].lehneAb("kaputt");
    await tick();
    expect(s).toContainEqual({ art: "bild", url: "asset://localhost/cache/1.jpg" });
    expect(s).toContainEqual({ art: "nurOnline" });
    expect(s).toContainEqual({ art: "fehler", meldung: "kaputt" });
    expect(inv).toHaveBeenCalledWith("vorschau", { pfad: "/a.jpg", groesse: 600 });
  });
});

describe("useVorschau (F-LOG-10/11)", () => {
  it("lädt, zeigt Bild und merkt es sich (F-LOG-10)", async () => {
    const { result, unmount } = renderHook(() => useVorschau("/x.jpg"));
    expect(result.current.status).toEqual({ art: "laedt" });
    expect(inv).toHaveBeenCalledWith("vorschau", { pfad: "/x.jpg", groesse: VORSCHAU_GROESSE });
    await act(async () => offen[0].loese({ art: "bild", datei: "/c/x.jpg" }));
    await waitFor(() => expect(result.current.status.art).toBe("bild"));
    unmount();
    // zweites Mal sofort aus dem Speicher, ohne neuen Aufruf
    const zweit = renderHook(() => useVorschau("/x.jpg"));
    expect(zweit.result.current.status.art).toBe("bild");
    expect(inv).toHaveBeenCalledTimes(1);
  });

  it("nur online und Fehler werden nicht gemerkt; Unmount bricht ab (F-LOG-11)", async () => {
    const a = renderHook(() => useVorschau("/wolke.jpg"));
    await act(async () => offen[0].loese({ art: "nurOnline" }));
    await waitFor(() => expect(a.result.current.status.art).toBe("nurOnline"));
    a.unmount();
    renderHook(() => useVorschau("/wolke.jpg"));
    expect(inv).toHaveBeenCalledTimes(2); // erneut gefragt – z. B. nach Download

    _zuruecksetzen();
    inv.mockClear();
    for (let i = 0; i < MAX_PARALLEL; i++) anfordern(`/voll${i}.jpg`, 600, () => {});
    const b = renderHook(() => useVorschau("/weggescrollt.jpg"));
    b.unmount();
    offen.at(-1)!.loese({ art: "fehlt" });
    await tick();
    expect(inv.mock.calls.map((c) => (c[1] as { pfad: string }).pfad)).not.toContain("/weggescrollt.jpg");
  });
});

describe("nurOnlineNeuLaden (F-LOG-20)", () => {
  it("Wolken-Kacheln fragen erneut, gemerkte Bilder nicht", async () => {
    const wolke = renderHook(() => useVorschau("/wolke.jpg"));
    const bild = renderHook(() => useVorschau("/bild.jpg"));
    await act(async () => {
      offen[0].loese({ art: "nurOnline" });
      offen[1].loese({ art: "bild", datei: "/c/b.jpg" });
    });
    await waitFor(() => expect(wolke.result.current.status.art).toBe("nurOnline"));
    act(() => nurOnlineNeuLaden());
    expect(wolke.result.current.status.art).toBe("laedt");
    expect(bild.result.current.status.art).toBe("bild");
    expect(inv).toHaveBeenCalledTimes(3);
    expect(offen[2].pfad).toBe("/wolke.jpg");
    await act(async () => offen[2].loese({ art: "bild", datei: "/c/w.jpg" }));
    await waitFor(() => expect(wolke.result.current.status.art).toBe("bild"));
    wolke.unmount();
    bild.unmount();
  });
});

describe("Vorschau aus der DB und Cache leeren", () => {
  it("DB-Vorschau wird über vorschau:// geladen und gemerkt (F-LOG-22)", async () => {
    expect(VORSCHAU_GROESSE).toBe(640);
    expect(umsetzen({ art: "datenbank", schluessel: "abc#2" })).toEqual({ art: "bild", url: "vorschau://localhost/abc%232" });
    const h = renderHook(() => useVorschau("/db.jpg"));
    await act(async () => offen[0].loese({ art: "datenbank", schluessel: "abc" }));
    await waitFor(() => expect(h.result.current.status).toEqual({ art: "bild", url: "vorschau://localhost/abc" }));
    h.unmount();
    renderHook(() => useVorschau("/db.jpg"));
    expect(inv).toHaveBeenCalledTimes(1);
  });

  it("alleNeuLaden fordert auch gemerkte Bilder neu an (F-LOG-23)", async () => {
    const h = renderHook(() => useVorschau("/bild.jpg"));
    await act(async () => offen[0].loese({ art: "bild", datei: "/c/b.jpg" }));
    await waitFor(() => expect(h.result.current.status.art).toBe("bild"));
    act(() => alleNeuLaden());
    expect(h.result.current.status.art).toBe("laedt");
    expect(inv).toHaveBeenCalledTimes(2);
    h.unmount();
  });
});
