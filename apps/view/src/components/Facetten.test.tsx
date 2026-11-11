import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { Facette } from "../types";
import Facetten, { SICHTBAR } from "./Facetten";

const motiv: Facette = {
  feld: "motiv",
  titel: "Motiv",
  werte: [
    { wert: "Landschaft", anzahl: 1234 },
    { wert: "Porträt", anzahl: 56 },
  ],
};
const wetter: Facette = { feld: "wetter", titel: "Wetter", werte: [{ wert: "sonnig", anzahl: 4 }] };
const viele: Facette = {
  feld: "schlagworte",
  titel: "Schlagworte",
  werte: Array.from({ length: SICHTBAR + 5 }, (_, i) => ({ wert: `wort${i}`, anzahl: 100 - i })),
};
const oeffnen = (titel: string) => userEvent.click(screen.getByRole("button", { name: new RegExp(`^${titel}`) }));

describe("Facetten", () => {
  it("kompakt: eine Leiste mit einem Knopf je Facette, Werte erst nach Klick (F-UI-45)", async () => {
    render(<Facetten facetten={[motiv, wetter]} auswahl={{}} onChange={vi.fn()} />);
    expect(screen.getByRole("button", { name: /^Motiv/ })).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByRole("button", { name: /Landschaft/ })).toBeNull();
    await oeffnen("Motiv");
    expect(screen.getByRole("button", { name: /^Motiv/ })).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("button", { name: /Landschaft/ })).toHaveTextContent("1.234");
    await oeffnen("Wetter");
    expect(screen.queryByRole("button", { name: /Landschaft/ }), "nur eine Facette offen").toBeNull();
    expect(screen.getByRole("button", { name: /sonnig/ })).toBeInTheDocument();
  });

  it("Klick wählt einen Wert, erneuter Klick entfernt ihn; Knopf zeigt Anzahl gewählter Werte (F-UI-46)", async () => {
    const onChange = vi.fn();
    const { rerender } = render(<Facetten facetten={[motiv]} auswahl={{}} onChange={onChange} />);
    await oeffnen("Motiv");
    await userEvent.click(screen.getByRole("button", { name: /Porträt/ }));
    expect(onChange).toHaveBeenLastCalledWith({ motiv: ["Porträt"] });
    rerender(<Facetten facetten={[motiv]} auswahl={{ motiv: ["Porträt"], wetter: ["Regen"] }} onChange={onChange} />);
    expect(screen.getByRole("button", { name: /^Motiv/ })).toHaveClass("facette__knopf--aktiv");
    expect(screen.getByRole("button", { name: /^Motiv/ })).toHaveTextContent("1");
    expect(screen.getByRole("button", { name: /^Porträt \(/ })).toHaveClass("chip--aktiv");
    await userEvent.click(screen.getByRole("button", { name: /^Landschaft \(/ }));
    expect(onChange).toHaveBeenLastCalledWith({ motiv: ["Porträt", "Landschaft"], wetter: ["Regen"] });
    await userEvent.click(screen.getByRole("button", { name: /^Porträt \(/ }));
    expect(onChange).toHaveBeenLastCalledWith({ motiv: [], wetter: ["Regen"] });
  });

  it("gewählter Wert ohne Treffer bleibt sichtbar und wirkt gedämpft (F-UI-47)", async () => {
    const f: Facette = { ...motiv, werte: [...motiv.werte, { wert: "Tier", anzahl: 0 }] };
    render(<Facetten facetten={[f]} auswahl={{ motiv: ["Tier"] }} onChange={vi.fn()} />);
    await oeffnen("Motiv");
    expect(screen.getByRole("button", { name: /^Tier \(/ })).toHaveClass("chip--aktiv", "chip--leer");
  });

  it("lange Listen zeigen zuerst nur einen Teil, „weitere“ klappt auf (F-UI-48)", async () => {
    render(<Facetten facetten={[viele]} auswahl={{ schlagworte: [`wort${SICHTBAR + 2}`] }} onChange={vi.fn()} />);
    await oeffnen("Schlagworte");
    expect(screen.queryByRole("button", { name: new RegExp(`^wort${SICHTBAR + 1} \\(`) })).toBeNull();
    expect(screen.getByRole("button", { name: new RegExp(`^wort${SICHTBAR + 2} \\(`) })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /weitere/ }));
    expect(screen.getByRole("button", { name: new RegExp(`^wort${SICHTBAR + 1} \\(`) })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /weniger/ })).toBeInTheDocument();
  });

  it("Facetten ohne Werte werden nicht gezeigt (F-UI-49)", () => {
    render(<Facetten facetten={[{ feld: "wetter", titel: "Wetter", werte: [] }]} auswahl={{}} onChange={vi.fn()} />);
    expect(screen.queryByText("Wetter")).toBeNull();
  });

  it("gewählte Werte stehen als entfernbare Chips in der Leiste; Esc und Klick außerhalb schließen (F-UI-54)", async () => {
    const onChange = vi.fn();
    render(<Facetten facetten={[motiv, wetter]} auswahl={{ motiv: ["Porträt"] }} onChange={onChange} />);
    await userEvent.click(screen.getByRole("button", { name: "Porträt entfernen" }));
    expect(onChange).toHaveBeenLastCalledWith({ motiv: [] });
    await oeffnen("Wetter");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("button", { name: /sonnig/ })).toBeNull();
    await oeffnen("Wetter");
    fireEvent.mouseDown(document.body);
    expect(screen.queryByRole("button", { name: /sonnig/ })).toBeNull();
  });
});
