// Vorschau-Cache im Einstellungsdialog (bis v0.2 im DB-Menü: DbMenu.cache.test.tsx)
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("../api", async (orig) => ({
  ...(await orig<typeof import("../api")>()),
  api: {
    einstellungen: vi.fn(),
    dropboxStatus: vi.fn().mockRejectedValue("kein Tauri"),
    cacheInfo: vi.fn(),
    cacheLeeren: vi.fn(),
  },
}));
vi.mock("../vorschau", () => ({ nurOnlineNeuLaden: vi.fn(), alleNeuLaden: vi.fn() }));
import { api } from "../api";
import { alleNeuLaden } from "../vorschau";
import { einstellungen, info } from "../test/daten";
import EinstellungenDialog from "./EinstellungenDialog";

const a = vi.mocked(api);
beforeEach(() => {
  a.einstellungen.mockReset().mockResolvedValue(einstellungen);
  a.dropboxStatus.mockReset().mockRejectedValue("kein Tauri");
  a.cacheInfo.mockReset().mockResolvedValue({ bytes: 1.3 * 1024 ** 3, dateien: 24000 });
  a.cacheLeeren.mockReset().mockResolvedValue({ bytes: 0, dateien: 0 });
  vi.mocked(alleNeuLaden).mockReset();
});
const zeigen = () => render(<EinstellungenDialog db={info} onDbWechseln={vi.fn()} onSchliessen={vi.fn()} />);

describe("Einstellungen – Vorschau-Cache", () => {
  it("zeigt die Cache-Größe (F-UI-41)", async () => {
    zeigen();
    expect(await screen.findByText("belegt 1,3 GB")).toBeInTheDocument();
  });

  it("„Cache leeren“ leert, zeigt 0 B und lädt alle Vorschauen neu (F-UI-42)", async () => {
    zeigen();
    await userEvent.click(await screen.findByText("Cache leeren"));
    expect(a.cacheLeeren).toHaveBeenCalledOnce();
    expect(await screen.findByText("belegt 0 B")).toBeInTheDocument();
    expect(alleNeuLaden).toHaveBeenCalledOnce();
    expect(screen.getByText("Cache leeren")).toBeDisabled();
  });

  it("Größe nicht abfragbar → Anzeige und Knopf bleiben weg (F-UI-43)", async () => {
    a.cacheInfo.mockRejectedValue("kein Tauri");
    zeigen();
    await waitFor(() => expect(a.cacheInfo).toHaveBeenCalled());
    expect(screen.queryByText("Cache leeren")).toBeNull();
  });
});
