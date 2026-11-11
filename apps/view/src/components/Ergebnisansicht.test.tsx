import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fireEvent } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("react-virtuoso", () => import("../test/virtuosoMock"));
vi.mock("./Vorschaubild", () => ({ default: ({ pfad }: { pfad: string }) => <i data-pfad={pfad} /> }));
import { handles } from "../test/virtuosoMock";
import { ergebnis, treffer } from "../test/daten";
import Results from "./Results";
import ResultGrid from "./ResultGrid";
import ResultList, { ablage } from "./ResultList";

beforeEach(() => {
  handles.scrollIntoView.mockClear();
  handles.scrollToIndex.mockClear();
});

describe("Results", () => {
  it("keine Treffer (F-UI-18)", () => {
    render(<Results ergebnis={ergebnis(0)} auswahl={null} onAuswahl={vi.fn()} />);
    expect(screen.getByText("Keine Treffer.")).toBeInTheDocument();
  });

  it("Kopfzeile: Anzahl, Einzahl, Begrenzung, Dauer (F-UI-19)", () => {
    const { rerender } = render(<Results ergebnis={ergebnis(1)} auswahl={null} onAuswahl={vi.fn()} />);
    expect(screen.getByText(/^1 Bild/)).toBeInTheDocument();
    rerender(<Results ergebnis={ergebnis(3, 4200)} auswahl={null} onAuswahl={vi.fn()} />);
    expect(screen.getByText(/4\.200 Bilder/)).toHaveTextContent("die ersten 3 angezeigt");
    expect(screen.getByText(/7 ms/)).toBeInTheDocument();
  });

  it("Raster ist Standard; Umschalten auf Liste wird gemerkt (F-UI-20)", async () => {
    const { unmount } = render(<Results ergebnis={ergebnis(2)} auswahl={null} onAuswahl={vi.fn()} />);
    expect(screen.getByTestId("raster")).toBeInTheDocument();
    await userEvent.click(screen.getByTitle("Liste"));
    expect(screen.getByTestId("liste")).toBeInTheDocument();
    expect(screen.queryByTitle("Kachelgröße")).toBeNull();
    unmount();
    render(<Results ergebnis={ergebnis(2)} auswahl={null} onAuswahl={vi.fn()} />);
    expect(screen.getByTestId("liste")).toBeInTheDocument();
  });

  it("Kachelgröße wird gemerkt; kaputter Speicherwert → Standard (F-UI-21)", () => {
    localStorage.setItem("view.ansicht", "{kaputt");
    const { unmount } = render(<Results ergebnis={ergebnis(2)} auswahl={null} onAuswahl={vi.fn()} />);
    const regler = screen.getByTitle("Kachelgröße") as HTMLInputElement;
    expect(regler.value).toBe("180");
    fireEvent.change(regler, { target: { value: "250" } });
    expect(localStorage.getItem("view.kachel")).toBe("250");
    unmount();
    render(<Results ergebnis={ergebnis(2)} auswahl={null} onAuswahl={vi.fn()} />);
    expect((screen.getByTitle("Kachelgröße") as HTMLInputElement).value).toBe("250");
  });
});

describe("Results Mindestbreite (F-UI-70)", () => {
  it("meldet eine Kachel bzw. die Listenbreite, auch nach Umschalten", async () => {
    localStorage.clear();
    const f = vi.fn();
    render(<Results ergebnis={ergebnis(2)} auswahl={null} onAuswahl={vi.fn()} onMindestbreite={f} />);
    expect(f).toHaveBeenLastCalledWith(250); // Kopfzeile
    fireEvent.change(screen.getByTitle("Kachelgröße"), { target: { value: "300" } });
    expect(f).toHaveBeenLastCalledWith(300 + 28);
    await userEvent.click(screen.getByTitle("Liste"));
    expect(f).toHaveBeenLastCalledWith(420);
  });
});

describe("ResultGrid", () => {
  const liste = [treffer(1), treffer(2, { kurztitel: null, dateiname: "ohne-titel.jpg" }), treffer(3, { seite: 4 })];

  it("Kacheln mit Titel (Rückfall Dateiname), Datum, Seite und Vorschau (F-UI-22)", () => {
    render(<ResultGrid treffer={liste} auswahl={null} onAuswahl={vi.fn()} kachel={180} />);
    expect(screen.getByText("Bild 1")).toBeInTheDocument();
    expect(screen.getByText("ohne-titel.jpg")).toBeInTheDocument();
    expect(screen.getByText("09.06.2015 · S. 4")).toBeInTheDocument();
    expect(document.querySelector('[data-pfad="/Archiv/Reise/IMG_1.jpg"]')).not.toBeNull();
  });

  it("Klick wählt, Auswahl ist markiert (F-UI-23)", async () => {
    const waehle = vi.fn();
    render(<ResultGrid treffer={liste} auswahl={2} onAuswahl={waehle} kachel={180} />);
    expect(screen.getByText("ohne-titel.jpg").closest(".kachel")).toHaveClass("kachel--aktiv");
    await userEvent.click(screen.getByText("Bild 3"));
    expect(waehle).toHaveBeenCalledWith(3);
  });

  it("Spaltenzahl aus der Breite; Tastatur wählt, Auswahl wird sichtbar gehalten (F-UI-24)", () => {
    // jsdom misst nichts – Breite vorgeben: (830 − 12 + 14) / (180 + 14) → 4 Spalten
    const breite = vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(830);
    const waehle = vi.fn();
    const { rerender } = render(<ResultGrid treffer={liste} auswahl={null} onAuswahl={waehle} kachel={180} />);
    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(waehle).toHaveBeenLastCalledWith(1);
    rerender(<ResultGrid treffer={liste} auswahl={1} onAuswahl={waehle} kachel={180} />);
    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(waehle).toHaveBeenLastCalledWith(2);
    fireEvent.keyDown(window, { key: "ArrowDown" });
    expect(waehle).toHaveBeenLastCalledWith(3); // eine Zeile (4) weiter, begrenzt auf das letzte Bild
    rerender(<ResultGrid treffer={liste} auswahl={3} onAuswahl={waehle} kachel={180} />);
    expect(handles.scrollToIndex).toHaveBeenCalled();
    breite.mockRestore();
  });
});

describe("ResultList", () => {
  it("Zeilen mit Titel, Ablage, Seite, Datum und Typ (F-UI-25)", () => {
    render(
      <ResultList
        treffer={[treffer(1, { endung: ".heic", seite: 2, archivWurzel: "/Users/kim/Globetrotter", ordner: "2015 Thailand" })]}
        auswahl={null}
        onAuswahl={vi.fn()}
      />,
    );
    const z = screen.getByText("Bild 1").closest(".zeile") as HTMLElement;
    expect(within(z).getByText("HEIC")).toBeInTheDocument();
    expect(within(z).getByText("09.06.2015")).toBeInTheDocument();
    expect(z).toHaveTextContent("IMG_1.jpg · Seite 2 — Globetrotter/2015 Thailand");
  });

  it("Klick und ↓ wählen, Auswahl wird sichtbar gehalten (F-UI-26)", async () => {
    const waehle = vi.fn();
    const { rerender } = render(<ResultList treffer={[treffer(1), treffer(2)]} auswahl={1} onAuswahl={waehle} />);
    expect(screen.getByText("Bild 1").closest(".zeile")).toHaveClass("zeile--aktiv");
    expect(handles.scrollIntoView).toHaveBeenCalledWith({ index: 0, behavior: "auto" });
    fireEvent.keyDown(window, { key: "ArrowDown" });
    expect(waehle).toHaveBeenLastCalledWith(2);
    await userEvent.click(screen.getByText("Bild 2"));
    expect(waehle).toHaveBeenLastCalledWith(2);
    rerender(<ResultList treffer={[treffer(1), treffer(2)]} auswahl={null} onAuswahl={waehle} />);
  });

  it("ablage(): letzter Teil der Wurzel + Ordner (F-LOG-19)", () => {
    expect(ablage("/Users/kim/Globetrotter/", "2015 Thailand")).toBe("Globetrotter/2015 Thailand");
    expect(ablage("/Archiv", "")).toBe("Archiv");
    expect(ablage(null, "Ordner")).toBe("Ordner");
    expect(ablage(null, null)).toBe("");
  });
});
