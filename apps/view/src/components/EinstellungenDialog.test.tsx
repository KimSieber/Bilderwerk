// Einstellungsdialog (v0.3): Laden, Speicherorte bearbeiten, Übernehmen, Schließen
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("../api", async (orig) => ({
  ...(await orig<typeof import("../api")>()),
  api: {
    einstellungen: vi.fn(),
    einstellungenSetzen: vi.fn(),
    anbieterVorschlaege: vi.fn(),
    dropboxStatus: vi.fn().mockRejectedValue("kein Tauri"),
    cacheInfo: vi.fn().mockRejectedValue("kein Tauri"),
  },
}));
vi.mock("../vorschau", () => ({ nurOnlineNeuLaden: vi.fn(), alleNeuLaden: vi.fn() }));
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "../api";
import { einstellungen, info } from "../test/daten";
import EinstellungenDialog from "./EinstellungenDialog";

const a = vi.mocked(api);
beforeEach(() => {
  a.einstellungen.mockReset().mockResolvedValue(einstellungen);
  a.einstellungenSetzen.mockReset().mockImplementation(async (x) => ({ ...einstellungen, ...x }));
  a.anbieterVorschlaege.mockReset();
  vi.mocked(open).mockReset();
});
const zeigen = (p: Partial<Parameters<typeof EinstellungenDialog>[0]> = {}) =>
  render(<EinstellungenDialog db={info} onDbWechseln={vi.fn()} onSchliessen={vi.fn()} {...p} />);
const ordner = () => screen.getAllByLabelText(/^Ordner \d/).map((e) => (e as HTMLInputElement).value);
const arten = () => screen.getAllByLabelText(/^Art \d/).map((e) => (e as HTMLSelectElement).value);

describe("Einstellungsdialog", () => {
  it("zeigt DB, Cache-Grenze, Regeln und Schlüssel; Übernehmen erst nach Änderung (F-UI-85)", async () => {
    const wechseln = vi.fn();
    zeigen({ onDbWechseln: wechseln });
    expect(await screen.findByLabelText("Höchstens")).toHaveValue("2048");
    expect(screen.getByText(info.pfad)).toBeInTheDocument();
    expect(ordner()).toEqual(["/Users/kim/Dropbox", "/"]);
    expect(arten()).toEqual(["dropbox-klassisch", "lokal"]);
    expect(screen.getByLabelText("App-Schlüssel")).toHaveValue("sm36zb0y3puyff6");
    expect(screen.getByText("Übernehmen")).toBeDisabled();
    expect(screen.getByText("Schließen")).toBeInTheDocument();
    await userEvent.type(screen.getByLabelText("Höchstens"), "0");
    expect(screen.getByText("Übernehmen")).toBeEnabled();
    expect(screen.getByText("Abbrechen")).toBeInTheDocument();
    await userEvent.click(screen.getByText("Andere Datenbank…"));
    expect(wechseln).toHaveBeenCalledOnce();
  });

  it("ohne DB: Hinweis und „Datenbank auswählen…“ (F-UI-85)", async () => {
    a.einstellungen.mockResolvedValue({ ...einstellungen, dbPfad: null });
    zeigen({ db: null });
    expect(await screen.findByText("Noch keine Datenbank ausgewählt.")).toBeInTheDocument();
    expect(screen.getByText("Datenbank auswählen…")).toBeInTheDocument();
  });

  it("Regeln hinzufügen, Ordner wählen, Art ändern, entfernen → Übernehmen (F-UI-86)", async () => {
    vi.mocked(open).mockResolvedValue("/Volumes/NAS/Bilder");
    zeigen();
    await screen.findByLabelText("Höchstens");
    await userEvent.click(screen.getByText("Regel hinzufügen"));
    expect(ordner()).toEqual(["/Users/kim/Dropbox", "/", ""]);
    await userEvent.click(screen.getAllByTitle("Ordner wählen")[2]);
    await waitFor(() => expect(ordner()[2]).toBe("/Volumes/NAS/Bilder"));
    expect(vi.mocked(open).mock.calls[0][0]).toMatchObject({ directory: true });
    await userEvent.selectOptions(screen.getByLabelText("Art 3"), "file-provider");
    await userEvent.click(screen.getByLabelText("Regel 1 entfernen"));
    expect(ordner()).toEqual(["/", "/Volumes/NAS/Bilder"]);
    await userEvent.click(screen.getByText("Übernehmen"));
    expect(a.einstellungenSetzen).toHaveBeenCalledWith({
      cacheGrenzeMb: 2048,
      anbieter: [
        { pfad: "/", art: "lokal" },
        { pfad: "/Volumes/NAS/Bilder", art: "file-provider" },
      ],
      dropboxAppKey: "sm36zb0y3puyff6",
    });
    expect(await screen.findByText("Übernommen")).toBeInTheDocument();
    expect(screen.getByText("Übernehmen")).toBeDisabled();
  });

  it("Ordnerwahl abgebrochen ändert nichts (F-UI-86)", async () => {
    vi.mocked(open).mockResolvedValue(null);
    zeigen();
    await screen.findByLabelText("Höchstens");
    await userEvent.click(screen.getAllByTitle("Ordner wählen")[0]);
    expect(ordner()).toEqual(["/Users/kim/Dropbox", "/"]);
    expect(screen.getByText("Übernehmen")).toBeDisabled();
  });

  it("erkannte Ordner: nur neue Pfade werden ergänzt (F-UI-87)", async () => {
    a.anbieterVorschlaege.mockResolvedValue([
      { pfad: "/Users/kim/Dropbox", art: "dropbox-klassisch" },
      { pfad: "/Users/kim/Library/Mobile Documents", art: "file-provider" },
      { pfad: "/", art: "lokal" },
    ]);
    zeigen();
    await screen.findByLabelText("Höchstens");
    await userEvent.click(screen.getByText("Erkannte Ordner vorschlagen"));
    await waitFor(() => expect(ordner()).toHaveLength(3));
    expect(ordner()).toEqual(["/Users/kim/Dropbox", "/", "/Users/kim/Library/Mobile Documents"]);
    expect(arten()[2]).toBe("file-provider");
  });

  it("Fehler beim Übernehmen sichtbar, ungültige Cache-Grenze gar nicht erst gesendet (F-UI-88)", async () => {
    a.einstellungenSetzen.mockRejectedValue("Der Ordner einer Regel muss ein absoluter Pfad sein: \"x\"");
    zeigen();
    const grenze = await screen.findByLabelText("Höchstens");
    await userEvent.clear(grenze);
    await userEvent.type(grenze, "viel");
    await userEvent.click(screen.getByText("Übernehmen"));
    expect(screen.getByText(/ganze Zahl in MB/)).toBeInTheDocument();
    expect(a.einstellungenSetzen).not.toHaveBeenCalled();
    await userEvent.clear(grenze);
    await userEvent.type(grenze, "500");
    await userEvent.click(screen.getByText("Übernehmen"));
    expect(await screen.findByText(/absoluter Pfad/)).toBeInTheDocument();
    expect(screen.getByText("Übernehmen")).toBeEnabled();
  });

  it("Esc und Klick auf den Hintergrund schließen; andere Tastenhörer merken nichts (F-UI-90)", async () => {
    const zu = vi.fn();
    const andere = vi.fn();
    window.addEventListener("keydown", andere);
    const { unmount } = zeigen({ onSchliessen: zu });
    const grenze = await screen.findByLabelText("Höchstens");
    fireEvent.keyDown(grenze, { key: "ArrowDown" });
    fireEvent.keyDown(window, { key: "f", metaKey: true });
    expect(andere).not.toHaveBeenCalled();
    expect(zu).not.toHaveBeenCalled();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(zu).toHaveBeenCalledOnce();
    fireEvent.mouseDown(screen.getByText("Datenbank"));
    expect(zu).toHaveBeenCalledOnce();
    fireEvent.mouseDown(screen.getByRole("dialog"));
    expect(zu).toHaveBeenCalledTimes(2);
    unmount();
    fireEvent.keyDown(window, { key: "x" });
    expect(andere).toHaveBeenCalledOnce();
    window.removeEventListener("keydown", andere);
  });
});
