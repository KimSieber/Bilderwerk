import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useTastatur } from "./useTastatur";

function aufbau(o: { anzahl?: number; index?: number; spalten?: number } = {}) {
  const waehle = vi.fn();
  const el = { clientHeight: 500, scrollHeight: 5000, scrollBy: vi.fn(), scrollTo: vi.fn() };
  renderHook(() =>
    useTastatur({
      anzahl: o.anzahl ?? 20,
      index: o.index ?? 5,
      waehle,
      spalten: o.spalten ?? 4,
      scroller: () => el as unknown as HTMLElement,
    }),
  );
  return { waehle, el };
}
const taste = (key: string, mehr: KeyboardEventInit = {}) =>
  window.dispatchEvent(new KeyboardEvent("keydown", { key, cancelable: true, ...mehr }));

describe("useTastatur", () => {
  it("↑/↓ springen eine Zeile, begrenzt auf die Treffer (F-LOG-12)", () => {
    const { waehle } = aufbau({ index: 5, spalten: 4 });
    taste("ArrowDown");
    expect(waehle).toHaveBeenLastCalledWith(9);
    taste("ArrowUp");
    expect(waehle).toHaveBeenLastCalledWith(1);
    const r = aufbau({ index: 18, spalten: 4 });
    taste("ArrowDown");
    expect(r.waehle).toHaveBeenLastCalledWith(19);
  });

  it("ohne Auswahl wählt die erste Taste Bild 1 (F-LOG-13)", () => {
    const { waehle } = aufbau({ index: -1 });
    taste("ArrowUp");
    expect(waehle).toHaveBeenLastCalledWith(0);
  });

  it("←/→ nur im Raster (F-LOG-14)", () => {
    const g = aufbau({ index: 5, spalten: 4 });
    taste("ArrowRight");
    expect(g.waehle).toHaveBeenLastCalledWith(6);
    taste("ArrowLeft");
    expect(g.waehle).toHaveBeenLastCalledWith(4);
  });

  it("←/→ in der Liste ohne Wirkung (F-LOG-15)", () => {
    const l = aufbau({ index: 5, spalten: 1 });
    taste("ArrowRight");
    taste("ArrowLeft");
    expect(l.waehle).not.toHaveBeenCalled();
  });

  it("im Suchfeld: ↓ verlässt das Feld, ←/→ bewegen den Textcursor (F-LOG-16)", () => {
    const { waehle } = aufbau({ index: 5, spalten: 4 });
    const feld = document.createElement("input");
    document.body.append(feld);
    feld.focus();
    taste("ArrowRight");
    expect(waehle).not.toHaveBeenCalled();
    taste("ArrowDown");
    expect(waehle).toHaveBeenLastCalledWith(9);
    expect(document.activeElement).not.toBe(feld);
    feld.remove();
  });

  it("Bild↑/↓ blättern, Pos1/Ende springen (F-LOG-17)", () => {
    const { el, waehle } = aufbau();
    taste("PageDown");
    expect(el.scrollBy).toHaveBeenLastCalledWith({ top: 450, behavior: "smooth" });
    taste("PageUp");
    expect(el.scrollBy).toHaveBeenLastCalledWith({ top: -450, behavior: "smooth" });
    taste("Home");
    expect(el.scrollTo).toHaveBeenLastCalledWith({ top: 0, behavior: "smooth" });
    taste("End");
    expect(el.scrollTo).toHaveBeenLastCalledWith({ top: 5000, behavior: "smooth" });
    expect(waehle).not.toHaveBeenCalled();
  });

  it("mit ⌘/Ctrl/Alt oder ohne Treffer keine Wirkung (F-LOG-18)", () => {
    const a = aufbau();
    taste("ArrowDown", { metaKey: true });
    taste("ArrowDown", { ctrlKey: true });
    taste("ArrowDown", { altKey: true });
    expect(a.waehle).not.toHaveBeenCalled();
    const b = aufbau({ anzahl: 0 });
    taste("ArrowDown");
    expect(b.waehle).not.toHaveBeenCalled();
  });
});
