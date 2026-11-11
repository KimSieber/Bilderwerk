import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
import { invoke } from "@tauri-apps/api/core";
import { api, datumDe, groesseDe, zahlDe } from "./api";
import { LEERER_FILTER } from "./types";

const inv = vi.mocked(invoke);
beforeEach(() => inv.mockReset().mockResolvedValue("ok"));

describe("datumDe (F-LOG-01)", () => {
  it("wandelt ISO in deutsches Datum", () => {
    expect(datumDe("2015-06-09T14:12:36")).toBe("09.06.2015");
    expect(datumDe("2015-06-09")).toBe("09.06.2015");
  });
  it("leer bei null/leer", () => {
    expect(datumDe(null)).toBe("");
    expect(datumDe("")).toBe("");
  });
  it("unplausible Werte bleiben roh (gekürzt)", () => {
    expect(datumDe("2056-55-10T00:00:00")).toBe("2056-55-10");
    expect(datumDe("2015-00-01")).toBe("2015-00-01");
    expect(datumDe("2015-06-32")).toBe("2015-06-32");
    expect(datumDe("irgendwas")).toBe("irgendwas");
  });
});

describe("zahlDe (F-LOG-02)", () => {
  it("Tausenderpunkt", () => {
    expect(zahlDe(23742)).toBe("23.742");
    expect(zahlDe(0)).toBe("0");
    expect(zahlDe(1234567)).toBe("1.234.567");
  });
});

describe("api (F-LOG-03)", () => {
  it("ruft die Rust-Befehle mit den richtigen Namen und Argumenten", async () => {
    await api.start();
    expect(inv).toHaveBeenLastCalledWith("start");
    await api.dbSetzen("/x.db");
    expect(inv).toHaveBeenLastCalledWith("db_setzen", { pfad: "/x.db" });
    const f = { ...LEERER_FILTER, freitext: "baum" };
    await api.suchen(f);
    expect(inv).toHaveBeenLastCalledWith("suchen", { filter: f });
    await api.dropboxStatus();
    expect(inv).toHaveBeenLastCalledWith("dropbox_status");
    await api.dropboxAnmelden();
    expect(inv).toHaveBeenLastCalledWith("dropbox_anmelden");
    await api.dropboxAbmelden();
    expect(inv).toHaveBeenLastCalledWith("dropbox_abmelden");
    await api.cacheInfo();
    expect(inv).toHaveBeenLastCalledWith("cache_info");
    await api.cacheLeeren();
    expect(inv).toHaveBeenLastCalledWith("cache_leeren");
  });
  it("reicht Fehler durch", async () => {
    inv.mockRejectedValueOnce("Keine Datenbank geöffnet.");
    await expect(api.suchen(LEERER_FILTER)).rejects.toBe("Keine Datenbank geöffnet.");
  });
});

describe("groesseDe (F-LOG-21)", () => {
  it("Bytes lesbar mit deutschem Komma", () => {
    expect(groesseDe(0)).toBe("0 B");
    expect(groesseDe(12 * 1024)).toBe("12 KB");
    expect(groesseDe(840 * 1024 * 1024)).toBe("840 MB");
    expect(groesseDe(1.3 * 1024 ** 3)).toBe("1,3 GB");
    expect(groesseDe(2 * 1024 ** 3)).toBe("2 GB");
  });
});

describe("api Neu laden (F-LOG-30)", () => {
  it("db_neu_laden", async () => {
    await api.dbNeuLaden();
    expect(inv).toHaveBeenLastCalledWith("db_neu_laden");
  });
});

describe("api Detailbereich (F-LOG-26)", () => {
  it("Befehle für Details, große Vorschau und Aktionen", async () => {
    await api.details(7);
    expect(inv).toHaveBeenLastCalledWith("details", { id: 7 });
    await api.vorschauGross("/a.jpg");
    expect(inv).toHaveBeenLastCalledWith("vorschau_gross", { pfad: "/a.jpg" });
    await api.originalOeffnen("/a.jpg");
    expect(inv).toHaveBeenLastCalledWith("original_oeffnen", { pfad: "/a.jpg" });
    await api.imFinderZeigen("/a.jpg");
    expect(inv).toHaveBeenLastCalledWith("im_finder_zeigen", { pfad: "/a.jpg" });
    await api.dateinameVorschlag("/a.jpg");
    expect(inv).toHaveBeenLastCalledWith("dateiname_vorschlag", { pfad: "/a.jpg" });
    await api.sichernUnter("/a.jpg", "/z/b.jpg");
    expect(inv).toHaveBeenLastCalledWith("sichern_unter", { pfad: "/a.jpg", ziel: "/z/b.jpg" });
    await api.teilen("/a.jpg", [1, 2, 3, 4]);
    expect(inv).toHaveBeenLastCalledWith("teilen", { pfad: "/a.jpg", knopf: [1, 2, 3, 4] });
  });
});
