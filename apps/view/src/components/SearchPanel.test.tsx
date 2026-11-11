import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { LEERER_FILTER, type Facette, type SuchFilter } from "../types";
import { facetten as facettenDaten } from "../test/daten";
import SearchPanel from "./SearchPanel";

/** Kontrollierte Hülle wie in App: jede Änderung landet im Zustand und in `spion`. */
function Huelle({
  start = LEERER_FILTER,
  spion,
  facetten = [],
}: {
  start?: SuchFilter;
  spion: (f: SuchFilter) => void;
  facetten?: Facette[];
}) {
  const [f, setF] = useState(start);
  return (
    <SearchPanel
      filter={f}
      kompakt={false}
      facetten={facetten}
      onChange={(n) => {
        spion(n);
        setF(n);
      }}
    />
  );
}
const zuletzt = (spion: ReturnType<typeof vi.fn>) => spion.mock.calls.at(-1)![0] as SuchFilter;

describe("SearchPanel", () => {
  it("Suchfeld hat beim Start den Fokus, Eingabe ändert den Freitext (F-UI-01)", async () => {
    const spion = vi.fn();
    render(<Huelle spion={spion} />);
    const feld = screen.getByPlaceholderText("Bilder suchen …");
    expect(feld).toHaveFocus();
    await userEvent.type(feld, "baum");
    expect(zuletzt(spion).freitext).toBe("baum");
  });

  it("Kriterien auf- und zuklappen (F-UI-02)", async () => {
    const { container } = render(<Huelle spion={vi.fn()} />);
    const knopf = screen.getByTitle("Weitere Kriterien");
    expect(container.querySelector(".kriterien--offen")).toBeNull();
    await userEvent.click(knopf);
    expect(container.querySelector(".kriterien--offen")).not.toBeNull();
    await userEvent.click(knopf);
    expect(container.querySelector(".kriterien--offen")).toBeNull();
  });

  it("Felder Dateiname, Ordner, Datum und Duplikate setzen den Filter (F-UI-03)", async () => {
    const spion = vi.fn();
    render(<Huelle spion={spion} />);
    await userEvent.type(screen.getByLabelText("Dateiname"), "IMG");
    await userEvent.type(screen.getByLabelText("Ordner"), "Reise");
    await userEvent.type(screen.getByLabelText("Datum von"), "2015");
    await userEvent.type(screen.getByLabelText("bis"), "2016-03");
    await userEvent.click(screen.getByLabelText("Duplikate einbeziehen"));
    expect(zuletzt(spion)).toMatchObject({
      name: "IMG",
      ordner: "Reise",
      datumVon: "2015",
      datumBis: "2016-03",
      mitDuplikaten: true,
    });
  });

  it("Dateityp-Chip schaltet eine ganze Endungsgruppe (F-UI-04)", async () => {
    const spion = vi.fn();
    render(<Huelle spion={spion} />);
    const jpeg = screen.getByRole("button", { name: "JPEG" });
    await userEvent.click(jpeg);
    expect(zuletzt(spion).endungen).toEqual([".jpg", ".jpeg"]);
    expect(jpeg).toHaveClass("chip--aktiv");
    await userEvent.click(screen.getByRole("button", { name: "PDF" }));
    expect(zuletzt(spion).endungen).toEqual([".jpg", ".jpeg", ".pdf"]);
    await userEvent.click(jpeg);
    expect(zuletzt(spion).endungen).toEqual([".pdf"]);
    expect(jpeg).not.toHaveClass("chip--aktiv");
  });

  it("Zähler zeigt die Anzahl gesetzter Kriterien (F-UI-05)", () => {
    render(
      <Huelle
        spion={vi.fn()}
        start={{ ...LEERER_FILTER, name: "a", datumVon: "2015", endungen: [".pdf"], mitDuplikaten: true }}
      />,
    );
    expect(screen.getByTitle("Weitere Kriterien")).toHaveTextContent("Kriterien4");
  });

  it("× leert alles außer „Duplikate“ und klappt zu (F-UI-06)", async () => {
    const spion = vi.fn();
    const { container } = render(
      <Huelle spion={spion} start={{ ...LEERER_FILTER, freitext: "x", ordner: "o", endungen: [".png"], mitDuplikaten: true }} />,
    );
    await userEvent.click(screen.getByTitle("Weitere Kriterien"));
    await userEvent.click(screen.getByTitle("Suche leeren (Esc)"));
    expect(zuletzt(spion)).toEqual({ ...LEERER_FILTER, mitDuplikaten: true });
    expect(container.querySelector(".kriterien--offen")).toBeNull();
    expect(screen.queryByTitle("Suche leeren (Esc)")).toBeNull();
    expect(screen.getByPlaceholderText("Bilder suchen …")).toHaveFocus();
  });

  it("Esc im Suchfeld leert ebenfalls (F-UI-07)", async () => {
    const spion = vi.fn();
    render(<Huelle spion={spion} start={{ ...LEERER_FILTER, freitext: "baum" }} />);
    await userEvent.type(screen.getByPlaceholderText("Bilder suchen …"), "{Escape}");
    expect(zuletzt(spion).freitext).toBe("");
  });

  it("⌘F holt den Fokus ins Suchfeld (F-UI-08)", async () => {
    render(<Huelle spion={vi.fn()} />);
    const feld = screen.getByPlaceholderText("Bilder suchen …");
    feld.blur();
    expect(feld).not.toHaveFocus();
    fireEvent.keyDown(window, { key: "f", metaKey: true });
    expect(feld).toHaveFocus();
  });

  it("kompakt setzt die Klasse für die obere Position (F-UI-09)", () => {
    const { container } = render(<SearchPanel filter={LEERER_FILTER} onChange={vi.fn()} kompakt />);
    expect(container.querySelector(".suche--kompakt")).not.toBeNull();
  });

  it("Facetten im Kriterienbereich: Zähler und Leeren berücksichtigen die Auswahl (F-UI-53)", async () => {
    const spion = vi.fn();
    render(<Huelle spion={spion} facetten={facettenDaten} />);
    await userEvent.click(screen.getByTitle("Weitere Kriterien"));
    await userEvent.click(screen.getByRole("button", { name: /^Motiv/ }));
    await userEvent.click(screen.getByRole("button", { name: /^Landschaft/ }));
    await userEvent.click(screen.getByRole("button", { name: /^Merkmale/ }));
    await userEvent.click(screen.getByRole("button", { name: /^brille/ }));
    expect(zuletzt(spion).facetten).toEqual({ motiv: ["Landschaft"], merkmale: ["brille"] });
    expect(screen.getByTitle("Weitere Kriterien")).toHaveTextContent("2");
    await userEvent.click(screen.getByTitle("Suche leeren (Esc)"));
    expect(zuletzt(spion).facetten).toEqual({});
  });
});
