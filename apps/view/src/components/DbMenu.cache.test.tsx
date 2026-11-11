import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({
  ...(await orig<typeof import("../api")>()),
  api: {
    dropboxStatus: vi.fn().mockRejectedValue("kein Tauri"),
    cacheInfo: vi.fn(),
    cacheLeeren: vi.fn(),
  },
}));
vi.mock("../vorschau", () => ({ nurOnlineNeuLaden: vi.fn(), alleNeuLaden: vi.fn() }));
import { api } from "../api";
import { alleNeuLaden } from "../vorschau";
import { info } from "../test/daten";
import DbMenu from "./DbMenu";

const a = vi.mocked(api);
beforeEach(() => {
  a.dropboxStatus.mockReset().mockRejectedValue("kein Tauri");
  a.cacheInfo.mockReset().mockResolvedValue({ bytes: 1.3 * 1024 ** 3, dateien: 24000 });
  a.cacheLeeren.mockReset().mockResolvedValue({ bytes: 0, dateien: 0 });
  vi.mocked(alleNeuLaden).mockReset();
});
const oeffnen = () => userEvent.click(screen.getByTitle("Datenbank"));

describe("DbMenu – Vorschau-Cache", () => {
  it("zeigt die Cache-Größe (F-UI-41)", async () => {
    render(<DbMenu info={info} onWechseln={vi.fn()} />);
    await oeffnen();
    expect(await screen.findByText("1,3 GB")).toBeInTheDocument();
  });

  it("„Cache leeren“ leert, zeigt 0 B und lädt alle Vorschauen neu (F-UI-42)", async () => {
    render(<DbMenu info={info} onWechseln={vi.fn()} />);
    await oeffnen();
    await userEvent.click(await screen.findByText("Cache leeren"));
    expect(a.cacheLeeren).toHaveBeenCalledOnce();
    expect(await screen.findByText("0 B")).toBeInTheDocument();
    expect(alleNeuLaden).toHaveBeenCalledOnce();
    expect(screen.getByText("Cache leeren")).toBeDisabled();
  });

  it("Größe nicht abfragbar → Bereich bleibt weg (F-UI-43)", async () => {
    a.cacheInfo.mockRejectedValue("kein Tauri");
    render(<DbMenu info={info} onWechseln={vi.fn()} />);
    await oeffnen();
    await waitFor(() => expect(a.cacheInfo).toHaveBeenCalled());
    expect(screen.queryByText("Cache leeren")).toBeNull();
  });
});
