import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../vorschau", () => ({ useVorschau: vi.fn() }));
import { useVorschau, type VorschauStatus } from "../vorschau";
import Vorschaubild from "./Vorschaubild";

const setze = (status: VorschauStatus) => vi.mocked(useVorschau).mockReturnValue({ status });
beforeEach(() => vi.mocked(useVorschau).mockReset());

describe("Vorschaubild", () => {
  it("Bild wird nach dem Laden weich eingeblendet (F-UI-14)", () => {
    setze({ art: "bild", url: "asset://x.jpg" });
    const { container } = render(<Vorschaubild pfad="/x.jpg" />);
    const img = container.querySelector("img")!;
    expect(img).toHaveAttribute("src", "asset://x.jpg");
    expect(img).not.toHaveClass("geladen");
    fireEvent.load(img);
    expect(img).toHaveClass("geladen");
    expect(useVorschau).toHaveBeenCalledWith("/x.jpg");
  });

  it("nur online → Wolke (F-UI-15)", () => {
    setze({ art: "nurOnline" });
    const { container } = render(<Vorschaubild pfad="/x.jpg" />);
    expect(container.querySelector(".vorschau__symbol")?.getAttribute("title")).toMatch(/Nur online/);
  });

  it("fehlt und Fehler → Symbol mit Hinweis (F-UI-16)", () => {
    setze({ art: "fehlt" });
    render(<Vorschaubild pfad="/x.jpg" />);
    expect(screen.getByTitle("Datei nicht gefunden")).toBeInTheDocument();
    setze({ art: "fehler", meldung: "sips kaputt" });
    render(<Vorschaubild pfad="/y.jpg" />);
    expect(screen.getByTitle("sips kaputt")).toBeInTheDocument();
  });

  it("während des Ladens bleibt die Fläche leer (F-UI-17)", () => {
    setze({ art: "laedt" });
    const { container } = render(<Vorschaubild pfad="/x.jpg" />);
    expect(container).toBeEmptyDOMElement();
  });
});
