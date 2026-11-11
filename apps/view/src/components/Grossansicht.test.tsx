import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({
  ...(await orig<typeof import("../api")>()),
  api: { vorschauGross: vi.fn() },
}));
vi.mock("@tauri-apps/api/core", () => ({ convertFileSrc: (p: string, s = "asset") => `${s}://localhost${p}`, invoke: vi.fn() }));
vi.mock("./Vorschaubild", () => ({ default: ({ pfad }: { pfad: string }) => <i data-testid="vorschau640" data-pfad={pfad} /> }));
import { api } from "../api";
import { treffer } from "../test/daten";
import Grossansicht from "./Grossansicht";

const a = vi.mocked(api);
beforeEach(() => a.vorschauGross.mockReset().mockImplementation(async (p) => ({ art: "bild", datei: `/c/gross${p}` })));
const voll = () => waitFor(() => {
  const i = document.querySelector(".grossansicht__voll");
  if (!i) throw new Error("kein Bild");
  return i as HTMLImageElement;
});
const f = () => ({ onBlaettern: vi.fn(), onSchliessen: vi.fn() });

describe("Großansicht", () => {
  it("zeigt sofort 640 px, blendet 2048 px ein; Titel und Position (F-UI-71)", async () => {
    render(<Grossansicht treffer={treffer(1)} position={[3, 1200]} {...f()} />);
    expect(screen.getByRole("dialog", { name: "Großansicht" })).toBeInTheDocument();
    expect(screen.getByTestId("vorschau640")).toHaveAttribute("data-pfad", "/Archiv/Reise/IMG_1.jpg");
    expect(screen.getByText("Lade große Fassung …")).toBeInTheDocument();
    expect(screen.getByText("Bild 1")).toBeInTheDocument();
    expect(screen.getByText("3 / 1.200")).toBeInTheDocument();
    const bild = await voll();
    expect(bild).toHaveAttribute("src", "asset://localhost/c/gross/Archiv/Reise/IMG_1.jpg");
    fireEvent.load(bild);
    expect(bild).toHaveClass("geladen");
    expect(screen.getByText("2048 px")).toBeInTheDocument();
    expect(a.vorschauGross).toHaveBeenCalledWith("/Archiv/Reise/IMG_1.jpg");
  });

  it("Blättern lädt neu, nur die jüngste Antwort zählt (F-UI-72)", async () => {
    let alt: (v: { art: "bild"; datei: string }) => void = () => {};
    a.vorschauGross.mockImplementationOnce(() => new Promise((r) => (alt = r)));
    const { rerender } = render(<Grossansicht treffer={treffer(1)} position={[1, 2]} {...f()} />);
    rerender(<Grossansicht treffer={treffer(2)} position={[2, 2]} {...f()} />);
    const img = await voll();
    await act(async () => alt({ art: "bild", datei: "/alt.jpg" }));
    expect(img).toHaveAttribute("src", "asset://localhost/c/gross/Archiv/Reise/IMG_2.jpg");
  });

  it("ohne große Fassung bleibt 640 px mit Hinweis (F-UI-73)", async () => {
    a.vorschauGross.mockResolvedValueOnce({ art: "nurOnline" });
    const { rerender } = render(<Grossansicht treffer={treffer(1)} position={[1, 2]} {...f()} />);
    expect(await screen.findByText(/bei Dropbox anmelden/)).toBeInTheDocument();
    a.vorschauGross.mockRejectedValueOnce("sips kaputt");
    rerender(<Grossansicht treffer={treffer(2)} position={[2, 2]} {...f()} />);
    expect(await screen.findByText(/sips kaputt/)).toBeInTheDocument();
    expect(screen.getByTestId("vorschau640")).toBeInTheDocument();
  });

  it("Tasten: ←/↑ zurück, →/↓ weiter, Esc/Leertaste schließen – andere Tastenhörer merken nichts (F-UI-74)", async () => {
    const x = f();
    const andere = vi.fn();
    window.addEventListener("keydown", andere);
    render(<Grossansicht treffer={treffer(1)} position={[1, 2]} {...x} />);
    for (const key of ["ArrowLeft", "ArrowUp"]) fireEvent.keyDown(document.body, { key });
    for (const key of ["ArrowRight", "ArrowDown"]) fireEvent.keyDown(document.body, { key });
    expect(x.onBlaettern.mock.calls).toEqual([[-1], [-1], [1], [1]]);
    fireEvent.keyDown(document.body, { key: "Escape" });
    fireEvent.keyDown(document.body, { key: " " });
    expect(x.onSchliessen).toHaveBeenCalledTimes(2);
    expect(andere).not.toHaveBeenCalled();
    fireEvent.keyDown(document.body, { key: "a" });
    expect(andere).toHaveBeenCalledOnce(); // übrige Tasten unberührt
    window.removeEventListener("keydown", andere);
  });

  it("Klick auf den Hintergrund oder × schließt, Klick in die Kopfzeile nicht (F-UI-75)", async () => {
    const x = f();
    render(<Grossansicht treffer={treffer(1)} position={[1, 2]} {...x} />);
    await userEvent.click(screen.getByText("Bild 1"));
    expect(x.onSchliessen).not.toHaveBeenCalled();
    await userEvent.click(screen.getByLabelText("Schließen"));
    expect(x.onSchliessen).toHaveBeenCalledTimes(1);
    await userEvent.click(screen.getByRole("dialog"));
    expect(x.onSchliessen).toHaveBeenCalledTimes(2);
  });
});
