import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import Teiler from "./Teiler";

describe("Teiler (F-UI-69)", () => {
  it("Ziehen meldet die Breite ab rechter Kante, Loslassen merkt, Doppelklick setzt zurück", () => {
    const f = { onZiehen: vi.fn(), onFertig: vi.fn(), onZuruecksetzen: vi.fn() };
    render(<Teiler bereich={() => ({ getBoundingClientRect: () => ({ right: 1000 }) }) as HTMLElement} {...f} />);
    const t = screen.getByRole("separator");
    fireEvent.pointerMove(t, { clientX: 500 });
    expect(f.onZiehen).not.toHaveBeenCalled(); // ohne Drücken nichts
    fireEvent.pointerDown(t, { clientX: 600 });
    expect(document.body.classList.contains("zieht")).toBe(true);
    fireEvent.pointerMove(t, { clientX: 600 });
    expect(f.onZiehen).toHaveBeenLastCalledWith(1000 - 600 - 7);
    fireEvent.pointerUp(t);
    expect(f.onFertig).toHaveBeenCalledOnce();
    expect(document.body.classList.contains("zieht")).toBe(false);
    fireEvent.doubleClick(t);
    expect(f.onZuruecksetzen).toHaveBeenCalledOnce();
  });
});
