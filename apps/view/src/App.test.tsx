import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", async (orig) => ({
  ...(await orig<typeof import("./api")>()),
  api: {
    start: vi.fn(),
    dbSetzen: vi.fn(),
    suchen: vi.fn(),
    facetten: vi.fn(),
    dropboxStatus: vi.fn().mockResolvedValue({ eingerichtet: false, angemeldet: false }),
    cacheInfo: vi.fn().mockResolvedValue({ bytes: 0, dateien: 0 }),
    details: vi.fn(),
    vorschauGross: vi.fn(),
    einstellungen: vi.fn(),
  },
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
// Tauri-Ereignisse: Rückrufe merken, der Test löst sie selbst aus
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const hoerer = vi.hoisted(() => new Map<string, (e?: any) => void>());
vi.mock("@tauri-apps/api/event", () => ({
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  listen: vi.fn(async (name: string, f: (e?: any) => void) => {
    hoerer.set(name, f);
    return () => hoerer.delete(name);
  }),
}));
vi.mock("./vorschau", async (orig) => ({ ...(await orig<typeof import("./vorschau")>()), alleNeuLaden: vi.fn() }));
import { alleNeuLaden } from "./vorschau";
vi.mock("react-virtuoso", () => import("./test/virtuosoMock"));
vi.mock("./components/Vorschaubild", () => ({ default: () => null }));
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "./api";
import App from "./App";
import { einstellungen, ergebnis, facetten, info } from "./test/daten";

const a = vi.mocked(api);
beforeEach(() => {
  a.start.mockReset().mockResolvedValue(info);
  a.dbSetzen.mockReset().mockResolvedValue(info);
  a.suchen.mockReset().mockResolvedValue(ergebnis(2));
  a.facetten.mockReset().mockResolvedValue(facetten);
  vi.mocked(open).mockReset();
  a.details.mockReset().mockImplementation(async (id: number) => ({
    id, pfad: `/Archiv/Reise/IMG_${id}.jpg`, felder: [{ name: "kurztitel", wert: `Detail ${id}` }], begriffe: {}, duplikate: [],
  }));
  a.vorschauGross.mockReset().mockResolvedValue({ art: "fehlt" });
  localStorage.clear();
});
const suchfeld = () => screen.findByPlaceholderText("Bilder suchen …");

describe("App", () => {
  it("ohne DB: Hinweis und Auswahl; Wahl öffnet die DB (F-UI-27)", async () => {
    a.start.mockRejectedValue("Noch keine Datenbank ausgewählt.");
    vi.mocked(open).mockResolvedValue("/neu.db");
    render(<App />);
    expect(await screen.findByText("Noch keine Datenbank ausgewählt.")).toBeInTheDocument();
    await userEvent.click(screen.getByText("Bildkatalog auswählen…"));
    expect(a.dbSetzen).toHaveBeenCalledWith("/neu.db");
    expect(await suchfeld()).toBeInTheDocument();
  });

  it("Dateiauswahl abgebrochen → nichts passiert (F-UI-28)", async () => {
    a.start.mockRejectedValue("x");
    vi.mocked(open).mockResolvedValue(null);
    render(<App />);
    await userEvent.click(await screen.findByText("Bildkatalog auswählen…"));
    expect(a.dbSetzen).not.toHaveBeenCalled();
  });

  it("ungültige DB gewählt → Fehlermeldung bleibt sichtbar (F-UI-29)", async () => {
    a.start.mockRejectedValue("x");
    a.dbSetzen.mockRejectedValue("Die Datei ist keine Bildkatalog-Datenbank (Tabellen fehlen).");
    vi.mocked(open).mockResolvedValue("/falsch.db");
    render(<App />);
    await userEvent.click(await screen.findByText("Bildkatalog auswählen…"));
    expect(await screen.findByText(/keine Bildkatalog-Datenbank/)).toBeInTheDocument();
  });

  it("Eingabe sucht verzögert und einmal, zeigt Treffer (F-UI-30)", async () => {
    render(<App />);
    await userEvent.type(await suchfeld(), "boot");
    expect(a.suchen).not.toHaveBeenCalled(); // Verzögerung 220 ms
    await waitFor(() => expect(a.suchen).toHaveBeenCalledTimes(1));
    expect(a.suchen.mock.calls[0][0].freitext).toBe("boot");
    expect(await screen.findByText(/2 Bilder/)).toBeInTheDocument();
    expect(document.querySelector(".app--suche")).not.toBeNull();
  });

  it("nur die jüngste Antwort zählt (F-UI-31)", async () => {
    let alt!: (e: ReturnType<typeof ergebnis>) => void;
    a.suchen
      .mockImplementationOnce(() => new Promise((r) => (alt = r)))
      .mockResolvedValueOnce(ergebnis(5));
    render(<App />);
    const feld = await suchfeld();
    await userEvent.type(feld, "ba");
    await waitFor(() => expect(a.suchen).toHaveBeenCalledTimes(1));
    await userEvent.type(feld, "um");
    await waitFor(() => expect(a.suchen).toHaveBeenCalledTimes(2));
    expect(await screen.findByText(/5 Bilder/)).toBeInTheDocument();
    await act(async () => alt(ergebnis(1)));
    expect(screen.queryByText(/^1 Bild/)).toBeNull();
    expect(screen.getByText(/5 Bilder/)).toBeInTheDocument();
  });

  it("Suchfehler wird angezeigt (F-UI-32)", async () => {
    a.suchen.mockRejectedValue("Suchfehler: kaputt");
    render(<App />);
    await userEvent.type(await suchfeld(), "x");
    expect(await screen.findByText("Suchfehler: kaputt")).toBeInTheDocument();
  });

  it("Leeren führt zur leeren Startfläche zurück (F-UI-33)", async () => {
    render(<App />);
    await userEvent.type(await suchfeld(), "boot");
    await screen.findByText(/2 Bilder/);
    await userEvent.click(screen.getByTitle("Suche leeren (Esc)"));
    await waitFor(() => expect(document.querySelector(".app--leer")).not.toBeNull());
    expect(screen.queryByText(/2 Bilder/)).toBeNull();
  });

  it("DB-Wechsel über das Menü (F-UI-34)", async () => {
    vi.mocked(open).mockResolvedValue("/andere.db");
    render(<App />);
    await userEvent.click(await screen.findByTitle("Datenbank"));
    await userEvent.click(screen.getByText("Andere Datenbank…"));
    await waitFor(() => expect(a.dbSetzen).toHaveBeenCalledWith("/andere.db"));
  });

  it("„Vorschau-Cache leeren“ aus dem App-Menü lädt alle Vorschauen neu (F-UI-44)", async () => {
    const { unmount } = render(<App />);
    await suchfeld();
    await waitFor(() => expect(hoerer.has("cache-geleert")).toBe(true));
    hoerer.get("cache-geleert")!();
    expect(alleNeuLaden).toHaveBeenCalledOnce();
    unmount();
    expect(hoerer.has("cache-geleert")).toBe(false);
  });

  it("Einstellungen: App-Menü (⌘,) öffnet den Dialog, neue Speicherorte laden Vorschauen neu (F-UI-92)", async () => {
    a.einstellungen.mockResolvedValue(einstellungen);
    render(<App />);
    await suchfeld();
    await waitFor(() => expect(hoerer.has("einstellungen-oeffnen")).toBe(true));
    act(() => hoerer.get("einstellungen-oeffnen")!());
    expect(await screen.findByRole("dialog", { name: "Einstellungen" })).toBeInTheDocument();
    expect(screen.getByText(info.pfad)).toBeInTheDocument();
    await userEvent.click(screen.getByText("Schließen"));
    expect(screen.queryByRole("dialog")).toBeNull();
    hoerer.get("speicher-neu")!();
    expect(alleNeuLaden).toHaveBeenCalledOnce();
  });

  it("ohne DB: „Einstellungen…“ auf der Startfläche (F-UI-92)", async () => {
    a.start.mockRejectedValue("Noch keine Datenbank ausgewählt.");
    a.einstellungen.mockResolvedValue({ ...einstellungen, dbPfad: null });
    render(<App />);
    await userEvent.click(await screen.findByText("Einstellungen…"));
    expect(await screen.findByText("Noch keine Datenbank ausgewählt.", { selector: ".dialog__pfad" })).toBeInTheDocument();
  });

  it("Facetten werden auch ohne Suche geladen; Auswahl startet die Suche (F-UI-51)", async () => {
    render(<App />);
    await suchfeld();
    await waitFor(() => expect(a.facetten).toHaveBeenCalled());
    expect(a.suchen).not.toHaveBeenCalled();
    await userEvent.click(screen.getByTitle("Weitere Kriterien"));
    await userEvent.click(await screen.findByRole("button", { name: /^Motiv/ }));
    await userEvent.click(screen.getByRole("button", { name: /^Porträt/ }));
    await waitFor(() => expect(a.suchen).toHaveBeenCalledTimes(1));
    expect(a.suchen.mock.calls[0][0].facetten).toEqual({ motiv: ["Porträt"] });
    await waitFor(() => expect(a.facetten.mock.calls.at(-1)![0].facetten).toEqual({ motiv: ["Porträt"] }));
  });

  it("Fehler beim Laden der Facetten stört die Suche nicht (F-UI-52)", async () => {
    a.facetten.mockRejectedValue("Facetten: kaputt");
    render(<App />);
    await userEvent.type(await suchfeld(), "boot");
    expect(await screen.findByText(/2 Bilder/)).toBeInTheDocument();
  });

  it("ohne Auswahl keine Detailfläche; Klick und Pfeiltasten zeigen die Details rechts (F-UI-65)", async () => {
    render(<App />);
    await userEvent.type(await suchfeld(), "boot");
    await screen.findByText(/2 Bilder/);
    expect(screen.queryByLabelText("Details")).toBeNull(); // ohne Auswahl keine Fläche
    expect(screen.queryByRole("separator")).toBeNull();
    await userEvent.click(screen.getByText("Bild 1"));
    expect(screen.getByRole("separator")).toBeInTheDocument();
    expect(await screen.findByText("Detail 1")).toBeInTheDocument();
    expect(screen.getByText("1 / 2")).toBeInTheDocument();
    await userEvent.keyboard("{ArrowDown}"); // jsdom ohne Breite → eine Spalte
    expect(await screen.findByText("Detail 2")).toBeInTheDocument();
    expect(screen.getByText("2 / 2")).toBeInTheDocument();
  });

  it("Detailbereich ein-/ausblenden wird gemerkt (F-UI-66)", async () => {
    const { unmount } = render(<App />);
    await userEvent.type(await suchfeld(), "boot");
    await screen.findByText(/2 Bilder/);
    await userEvent.click(screen.getByText("Bild 1"));
    expect(await screen.findByLabelText("Details")).toBeInTheDocument();
    await userEvent.click(screen.getByTitle("Details ausblenden"));
    expect(screen.queryByLabelText("Details")).toBeNull();
    expect(localStorage.getItem("view.details")).toBe("false");
    unmount();
    render(<App />);
    await userEvent.type(await suchfeld(), "boot");
    await screen.findByText(/2 Bilder/);
    expect(screen.getByTitle("Details einblenden")).toBeInTheDocument();
  });

  it("Suchzeile unverändert bedienbar bei offenem Detailbereich (F-UI-67)", async () => {
    render(<App />);
    const feld = (await suchfeld()) as HTMLInputElement;
    await userEvent.type(feld, "boot");
    await screen.findByText(/2 Bilder/);
    await userEvent.click(screen.getByText("Bild 1"));
    await screen.findByText("Detail 1");
    // Leertaste und ←/→ im Suchfeld gehören dem Text, nicht dem Detailbereich
    feld.focus();
    await userEvent.keyboard(" see{ArrowLeft}{ArrowLeft}");
    expect(feld.value).toBe("boot see");
    expect(a.vorschauGross).not.toHaveBeenCalled();
    expect(screen.getByText("Detail 1")).toBeInTheDocument();
    await waitFor(() => expect(a.suchen.mock.calls.at(-1)![0].freitext).toBe("boot see"));
    // Kriterien und Facetten-Leiste wie gehabt; Esc schließt die Werte
    await userEvent.click(screen.getByTitle("Weitere Kriterien"));
    await userEvent.click(await screen.findByRole("button", { name: /^Motiv/ }));
    expect(screen.getByRole("button", { name: /^Porträt/ })).toBeInTheDocument();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("button", { name: /^Porträt/ })).toBeNull());
  });

  it("Leertaste öffnet die Großansicht, Pfeile blättern dort, Esc schließt ohne die Suche zu leeren (F-UI-76)", async () => {
    a.vorschauGross.mockResolvedValue({ art: "fehlt" });
    render(<App />);
    const feld = (await suchfeld()) as HTMLInputElement;
    await userEvent.type(feld, "boot");
    await screen.findByText(/2 Bilder/);
    feld.focus();
    await userEvent.keyboard(" ");
    expect(screen.queryByRole("dialog", { name: "Großansicht" })).toBeNull(); // im Suchfeld: Text
    await userEvent.click(screen.getAllByText("Bild 1")[0]);
    await userEvent.keyboard(" ");
    const dlg = await screen.findByRole("dialog", { name: "Großansicht" });
    expect(dlg).toHaveTextContent("1 / 2");
    await userEvent.keyboard("{ArrowRight}");
    expect(dlg).toHaveTextContent("2 / 2");
    expect(await screen.findByText("Detail 2")).toBeInTheDocument(); // Detailbereich folgt
    await userEvent.keyboard("{ArrowRight}");
    expect(dlg).toHaveTextContent("2 / 2"); // Ende
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("dialog", { name: "Großansicht" })).toBeNull();
    expect(feld.value).toBe("boot ");
    expect(screen.getByText(/2 Bilder/)).toBeInTheDocument();
  });

  it("Knopf „Großansicht“ im Detailbereich öffnet sie; ohne Auswahl keine (F-UI-77)", async () => {
    render(<App />);
    await userEvent.type(await suchfeld(), "boot");
    await screen.findByText(/2 Bilder/);
    (document.activeElement as HTMLElement).blur();
    await userEvent.keyboard(" ");
    expect(screen.queryByRole("dialog", { name: "Großansicht" })).toBeNull();
    await userEvent.click(screen.getByText("Bild 2"));
    await userEvent.click(screen.getByText("Großansicht"));
    expect(await screen.findByRole("dialog", { name: "Großansicht" })).toHaveTextContent("2 / 2");
  });

  it("Programmname „Bilderwerk View“ in der Titelleiste und bei der DB-Auswahl (F-UI-80)", async () => {
    a.start.mockRejectedValue("Noch keine Datenbank ausgewählt.");
    render(<App />);
    expect(await screen.findByRole("heading", { name: "Bilderwerk View" })).toBeInTheDocument();
    expect(document.querySelector(".titelleiste__name")).toHaveTextContent("Bilderwerk View");
  });

  it("„db-neu“: neue Info, Suche und Details frisch, Vorschauen neu, Auswahl bleibt, Meldung (F-UI-78)", async () => {
    render(<App />);
    await userEvent.type(await suchfeld(), "boot");
    await screen.findByText(/2 Bilder/);
    await userEvent.click(screen.getByText("Bild 2"));
    await screen.findByText("Detail 2");
    const suchen = a.suchen.mock.calls.length;
    const details = a.details.mock.calls.length;
    vi.mocked(alleNeuLaden).mockClear();
    await waitFor(() => expect(hoerer.has("db-neu")).toBe(true));
    await act(async () => hoerer.get("db-neu")!({ payload: { ...info, bilder: 24112, vorschauen: 5000 } }));
    expect(await screen.findByRole("status")).toHaveTextContent("Katalog aktualisiert · 24.112 Bilder");
    expect(alleNeuLaden).toHaveBeenCalledOnce();
    await waitFor(() => expect(a.suchen.mock.calls.length).toBe(suchen + 1));
    await waitFor(() => expect(a.details.mock.calls.length).toBe(details + 1));
    expect(screen.getByText("Detail 2")).toBeInTheDocument();
    await userEvent.click(screen.getByTitle("Datenbank"));
    expect(screen.getByText("24.112")).toBeInTheDocument();
  });

  it("Esc außerhalb des Suchfelds hebt die Auswahl auf; nicht bei offenem Facetten-Aufklapper (F-UI-79)", async () => {
    render(<App />);
    const feld = (await suchfeld()) as HTMLInputElement;
    await userEvent.type(feld, "boot");
    await screen.findByText(/2 Bilder/);
    await userEvent.click(screen.getByText("Bild 1"));
    expect(await screen.findByLabelText("Details")).toBeInTheDocument();
    await userEvent.click(screen.getByTitle("Weitere Kriterien"));
    await userEvent.click(await screen.findByRole("button", { name: /^Motiv/ }));
    (document.activeElement as HTMLElement).blur();
    await userEvent.keyboard("{Escape}");
    expect(screen.getByLabelText("Details")).toBeInTheDocument(); // erst schließt der Aufklapper
    await waitFor(() => expect(screen.queryByRole("button", { name: /^Porträt/ })).toBeNull());
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByLabelText("Details")).toBeNull();
    expect(feld.value).toBe("boot"); // Suche bleibt
  });
});

