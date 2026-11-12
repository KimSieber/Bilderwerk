import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { info } from "../test/daten";
import DbMenu from "./DbMenu";

describe("DbMenu", () => {
  it("öffnet und zeigt Bilder, Duplikate, Stand und Pfad (F-UI-10)", async () => {
    render(<DbMenu info={info} onWechseln={vi.fn()} onEinstellungen={vi.fn()} />);
    expect(screen.queryByText("Bilder")).toBeNull();
    await userEvent.click(screen.getByTitle("Datenbank"));
    expect(screen.getByText("23.742")).toBeInTheDocument();
    expect(screen.getByText("2.500")).toBeInTheDocument();
    expect(screen.getByText(info.pfad)).toBeInTheDocument();
    expect(screen.getByText(/2026/)).toBeInTheDocument(); // Stand aus Unix-Zeit
  });

  it("Vorschauen in der DB: Anzahl bzw. „keine“ (F-UI-40)", async () => {
    const { unmount } = render(<DbMenu info={info} onWechseln={vi.fn()} onEinstellungen={vi.fn()} />);
    await userEvent.click(screen.getByTitle("Datenbank"));
    expect(screen.getByText("Vorschauen in der DB").nextSibling).toHaveTextContent("keine");
    unmount();
    render(<DbMenu info={{ ...info, vorschauen: 24000 }} onWechseln={vi.fn()} onEinstellungen={vi.fn()} />);
    await userEvent.click(screen.getByTitle("Datenbank"));
    expect(screen.getByText("Vorschauen in der DB").nextSibling).toHaveTextContent("24.000");
  });

  it("ohne Änderungszeit „unbekannt“ (F-UI-11)", async () => {
    render(<DbMenu info={{ ...info, geaendert: null }} onWechseln={vi.fn()} onEinstellungen={vi.fn()} />);
    await userEvent.click(screen.getByTitle("Datenbank"));
    expect(screen.getByText("unbekannt")).toBeInTheDocument();
  });

  it("„Andere Datenbank…“ schließt und ruft den Wechsel (F-UI-12)", async () => {
    const wechseln = vi.fn();
    render(<DbMenu info={info} onWechseln={wechseln} onEinstellungen={vi.fn()} />);
    await userEvent.click(screen.getByTitle("Datenbank"));
    await userEvent.click(screen.getByText("Andere Datenbank…"));
    expect(wechseln).toHaveBeenCalledOnce();
    expect(screen.queryByText("Andere Datenbank…")).toBeNull();
  });

  it("Klick außerhalb schließt, Klick innen nicht (F-UI-13)", async () => {
    render(<DbMenu info={info} onWechseln={vi.fn()} onEinstellungen={vi.fn()} />);
    await userEvent.click(screen.getByTitle("Datenbank"));
    fireEvent.mouseDown(screen.getByText("Duplikate"));
    expect(screen.getByText("Duplikate")).toBeInTheDocument();
    fireEvent.mouseDown(document.body);
    expect(screen.queryByText("Duplikate")).toBeNull();
  });

  it("„Einstellungen…“ schließt und öffnet den Dialog (F-UI-84)", async () => {
    const einst = vi.fn();
    render(<DbMenu info={info} onWechseln={vi.fn()} onEinstellungen={einst} />);
    await userEvent.click(screen.getByTitle("Datenbank"));
    await userEvent.click(screen.getByText("Einstellungen…"));
    expect(einst).toHaveBeenCalledOnce();
    expect(screen.queryByText("Einstellungen…")).toBeNull();
  });
});
