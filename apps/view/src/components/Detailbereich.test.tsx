import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({
  ...(await orig<typeof import("../api")>()),
  api: {
    details: vi.fn(),
    vorschauGross: vi.fn(),
    originalOeffnen: vi.fn(),
    imFinderZeigen: vi.fn(),
    dateinameVorschlag: vi.fn(),
    sichernUnter: vi.fn(),
    teilen: vi.fn(),
  },
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ convertFileSrc: (p: string, s = "asset") => `${s}://localhost${p}`, invoke: vi.fn() }));
vi.mock("./Vorschaubild", () => ({ default: ({ pfad }: { pfad: string }) => <i data-testid="vorschau640" data-pfad={pfad} /> }));
import { save } from "@tauri-apps/plugin-dialog";
import { api } from "../api";
import type { Details } from "../types";
import { treffer } from "../test/daten";
import Detailbereich from "./Detailbereich";

const a = vi.mocked(api);
function details(id: number, extra: Partial<Details> = {}): Details {
  return {
    id,
    pfad: `/Archiv/Reise/IMG_${id}.jpg`,
    felder: [
      { name: "kurztitel", wert: `Titel ${id}` },
      { name: "beschreibung", wert: `Beschreibung ${id}` },
      { name: "aufnahme_datum", wert: "2019-07-14T21:42:00" },
      { name: "motiv", wert: "Landschaft" },
      { name: "status", wert: "ok" },
    ],
    begriffe: { schlagwort: ["strand", "meer"] },
    duplikate: [],
    ...extra,
  };
}
beforeEach(() => {
  a.details.mockReset().mockImplementation(async (id) => details(id));
  a.vorschauGross.mockReset().mockResolvedValue({ art: "bild", datei: "/c/gross/1.jpg" });
  for (const f of [a.originalOeffnen, a.imFinderZeigen, a.teilen]) f.mockReset().mockResolvedValue(undefined);
  a.dateinameVorschlag.mockReset().mockResolvedValue("IMG_1.jpg");
  a.sichernUnter.mockReset().mockResolvedValue("/Ziel/IMG_1.jpg");
  vi.mocked(save).mockReset();
});

describe("Detailbereich", () => {
  it("ohne Auswahl: Hinweis (F-UI-55)", () => {
    render(<Detailbereich treffer={null} position={null} />);
    expect(screen.getByText(/Bild auswählen/)).toBeInTheDocument();
    expect(a.details).not.toHaveBeenCalled();
  });

  it("Auswahl: 640-px-Vorschau, Titel, Beschreibung, Gruppen, Schlagworte, Position (F-UI-56)", async () => {
    render(<Detailbereich treffer={treffer(1)} position={[17, 312]} />);
    expect(screen.getByTestId("vorschau640")).toHaveAttribute("data-pfad", "/Archiv/Reise/IMG_1.jpg");
    expect(await screen.findByText("Titel 1")).toBeInTheDocument();
    expect(screen.getByText("Beschreibung 1")).toBeInTheDocument();
    expect(screen.getByText("14.07.2019 21:42")).toBeInTheDocument();
    expect(screen.getByText("Landschaft")).toBeInTheDocument();
    expect(screen.getByText("strand")).toBeInTheDocument();
    expect(screen.getByText("Analyse").closest("details")).not.toHaveAttribute("open");
    expect(screen.getByText("17 / 312")).toBeInTheDocument();
    expect(a.details).toHaveBeenCalledWith(1);
    expect(a.vorschauGross).not.toHaveBeenCalled(); // kein Nachladen ohne Wunsch
  });

  it("Blättern: nur die jüngste Antwort zählt (F-UI-57)", async () => {
    let langsam: (d: Details) => void = () => {};
    a.details.mockImplementationOnce(() => new Promise((r) => (langsam = r)));
    const { rerender } = render(<Detailbereich treffer={treffer(1)} position={[1, 2]} />);
    rerender(<Detailbereich treffer={treffer(2)} position={[2, 2]} />);
    expect(await screen.findByText("Titel 2")).toBeInTheDocument();
    await act(async () => langsam(details(1)));
    expect(screen.queryByText("Titel 1")).toBeNull();
  });

  it("Großansicht per Knopf und Doppelklick aufs Bild; kein Nachladen im Detailbereich (F-UI-58)", async () => {
    const onGross = vi.fn();
    render(<Detailbereich treffer={treffer(1)} position={[1, 2]} onGross={onGross} />);
    await userEvent.click(screen.getByText("Großansicht"));
    await userEvent.dblClick(screen.getByTestId("vorschau640"));
    expect(onGross).toHaveBeenCalledTimes(2);
    fireEvent.keyDown(window, { key: " " }); // Leertaste gehört der App, nicht dem Detailbereich
    expect(onGross).toHaveBeenCalledTimes(2);
    expect(a.vorschauGross).not.toHaveBeenCalled();
  });

  it("Aktionen: Öffnen, Finder, Teilen mit Knopfposition, Fehler sichtbar (F-UI-61)", async () => {
    render(<Detailbereich treffer={treffer(1)} position={[1, 1]} />);
    await userEvent.click(screen.getByText("Öffnen"));
    expect(a.originalOeffnen).toHaveBeenCalledWith("/Archiv/Reise/IMG_1.jpg");
    await userEvent.click(screen.getByText("Im Finder"));
    expect(a.imFinderZeigen).toHaveBeenCalledWith("/Archiv/Reise/IMG_1.jpg");
    await userEvent.click(screen.getByLabelText("Teilen"));
    expect(a.teilen).toHaveBeenCalledWith("/Archiv/Reise/IMG_1.jpg", [0, 0, 0, 0]);
    a.originalOeffnen.mockRejectedValueOnce("Datei liegt nur online – bitte bei Dropbox anmelden.");
    await userEvent.click(screen.getByText("Öffnen"));
    expect(await screen.findByText(/nur online/)).toBeInTheDocument();
  });

  it("Sichern unter: Dialog mit Vorschlag, Abbruch tut nichts (F-UI-62)", async () => {
    vi.mocked(save).mockResolvedValueOnce(null).mockResolvedValueOnce("/Ziel/IMG_1.jpg");
    render(<Detailbereich treffer={treffer(1)} position={[1, 1]} />);
    await userEvent.click(screen.getByText("Sichern unter …"));
    expect(save).toHaveBeenCalledWith(expect.objectContaining({ defaultPath: "IMG_1.jpg" }));
    expect(a.sichernUnter).not.toHaveBeenCalled();
    await userEvent.click(screen.getByText("Sichern unter …"));
    expect(a.sichernUnter).toHaveBeenCalledWith("/Archiv/Reise/IMG_1.jpg", "/Ziel/IMG_1.jpg");
    expect(await screen.findByText("Gesichert: /Ziel/IMG_1.jpg")).toBeInTheDocument();
  });

  it("Duplikat anklicken zeigt es an, „zurück“ und neue Auswahl kehren zurück (F-UI-63)", async () => {
    a.details.mockImplementation(async (id) =>
      id === 1 ? details(1, { duplikate: [{ id: 3, pfad: "/Sicherung/IMG_1.jpg", status: "duplikat" }] }) : details(id),
    );
    const { rerender } = render(<Detailbereich treffer={treffer(1)} position={[1, 2]} />);
    await userEvent.click(await screen.findByText("/Sicherung/IMG_1.jpg"));
    expect(await screen.findByText("Titel 3")).toBeInTheDocument();
    expect(screen.getByTestId("vorschau640")).toHaveAttribute("data-pfad", "/Sicherung/IMG_1.jpg");
    await userEvent.click(screen.getByText(/zurück zur Auswahl/));
    expect(await screen.findByText("Titel 1")).toBeInTheDocument();
    await userEvent.click(await screen.findByText("/Sicherung/IMG_1.jpg"));
    await screen.findByText("Titel 3");
    rerender(<Detailbereich treffer={treffer(2)} position={[2, 2]} />);
    expect(await screen.findByText("Titel 2")).toBeInTheDocument();
  });

  it("Fehler beim Laden der Angaben wird angezeigt (F-UI-64)", async () => {
    a.details.mockRejectedValueOnce("Bild 1 nicht im Katalog.");
    render(<Detailbereich treffer={treffer(1)} position={[1, 1]} />);
    expect(await screen.findByText("Bild 1 nicht im Katalog.")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText("Bild 1")).toBeInTheDocument()); // Titel aus dem Treffer
  });

  it("Tastenkürzel ⌘O, ⌥⌘R, ⇧⌘S (F-UI-82)", async () => {
    vi.mocked(save).mockResolvedValueOnce(null);
    render(<Detailbereich treffer={treffer(1)} position={[1, 1]} />);
    fireEvent.keyDown(window, { key: "o", metaKey: true });
    await waitFor(() => expect(a.originalOeffnen).toHaveBeenCalledWith("/Archiv/Reise/IMG_1.jpg"));
    fireEvent.keyDown(window, { key: "®", code: "KeyR", metaKey: true, altKey: true });
    await waitFor(() => expect(a.imFinderZeigen).toHaveBeenCalledWith("/Archiv/Reise/IMG_1.jpg"));
    fireEvent.keyDown(window, { key: "S", metaKey: true, shiftKey: true });
    await waitFor(() => expect(save).toHaveBeenCalled());
    fireEvent.keyDown(window, { key: "o" }); // ohne ⌘ nichts
    expect(a.originalOeffnen).toHaveBeenCalledTimes(1);
  });

  it("neuer DB-Stand holt die Angaben neu (F-UI-83)", async () => {
    const { rerender } = render(<Detailbereich treffer={treffer(1)} position={[1, 1]} stand={0} />);
    await screen.findByText("Titel 1");
    rerender(<Detailbereich treffer={treffer(1)} position={[1, 1]} stand={1} />);
    await waitFor(() => expect(a.details).toHaveBeenCalledTimes(2));
  });
});

