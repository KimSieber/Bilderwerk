// Dropbox-Anmeldung im Einstellungsdialog (bis v0.2 im DB-Menü: DbMenu.dropbox.test.tsx)
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("../api", async (orig) => ({
  ...(await orig<typeof import("../api")>()),
  api: {
    einstellungen: vi.fn(),
    dropboxStatus: vi.fn(),
    dropboxAnmelden: vi.fn(),
    dropboxAbmelden: vi.fn(),
    cacheInfo: vi.fn().mockRejectedValue("kein Tauri"),
  },
}));
vi.mock("../vorschau", () => ({ nurOnlineNeuLaden: vi.fn(), alleNeuLaden: vi.fn() }));
import { api } from "../api";
import { nurOnlineNeuLaden } from "../vorschau";
import { einstellungen, info } from "../test/daten";
import EinstellungenDialog from "./EinstellungenDialog";

const a = vi.mocked(api);
beforeEach(() => {
  a.einstellungen.mockReset().mockResolvedValue(einstellungen);
  a.dropboxStatus.mockReset().mockResolvedValue({ eingerichtet: true, angemeldet: false });
  a.dropboxAnmelden.mockReset().mockResolvedValue({ eingerichtet: true, angemeldet: true });
  a.dropboxAbmelden.mockReset().mockResolvedValue({ eingerichtet: true, angemeldet: false });
  vi.mocked(nurOnlineNeuLaden).mockReset();
});
const zeigen = () => render(<EinstellungenDialog db={info} onDbWechseln={vi.fn()} onSchliessen={vi.fn()} />);

describe("Einstellungen – Dropbox", () => {
  it("ohne App-Schlüssel keine Anmeldung (F-UI-35)", async () => {
    a.dropboxStatus.mockResolvedValue({ eingerichtet: false, angemeldet: false });
    zeigen();
    await waitFor(() => expect(a.dropboxStatus).toHaveBeenCalled());
    await screen.findByLabelText("App-Schlüssel");
    expect(screen.queryByText("Bei Dropbox anmelden…")).toBeNull();
    expect(screen.queryByText("nicht angemeldet")).toBeNull();
  });

  it("Status nicht angemeldet → Anmelden lädt Wolken neu (F-UI-36)", async () => {
    let fertig!: (s: { eingerichtet: boolean; angemeldet: boolean }) => void;
    a.dropboxAnmelden.mockImplementation(() => new Promise((r) => (fertig = r)));
    zeigen();
    expect(await screen.findByText("nicht angemeldet")).toBeInTheDocument();
    await userEvent.click(screen.getByText("Bei Dropbox anmelden…"));
    const knopf = screen.getByText("Anmeldung im Browser …");
    expect(knopf).toBeDisabled();
    fertig({ eingerichtet: true, angemeldet: true });
    expect(await screen.findByText("angemeldet")).toBeInTheDocument();
    expect(nurOnlineNeuLaden).toHaveBeenCalledOnce();
    expect(screen.getByText("Von Dropbox abmelden")).toBeInTheDocument();
  });

  it("Abmelden (F-UI-37)", async () => {
    a.dropboxStatus.mockResolvedValue({ eingerichtet: true, angemeldet: true });
    zeigen();
    await userEvent.click(await screen.findByText("Von Dropbox abmelden"));
    expect(a.dropboxAbmelden).toHaveBeenCalledOnce();
    expect(await screen.findByText("nicht angemeldet")).toBeInTheDocument();
    expect(nurOnlineNeuLaden).not.toHaveBeenCalled();
  });

  it("Fehler bei der Anmeldung wird angezeigt (F-UI-38)", async () => {
    a.dropboxAnmelden.mockRejectedValue("Zeit für die Anmeldung abgelaufen.");
    zeigen();
    await userEvent.click(await screen.findByText("Bei Dropbox anmelden…"));
    expect(await screen.findByText("Zeit für die Anmeldung abgelaufen.")).toBeInTheDocument();
    expect(screen.getByText("Bei Dropbox anmelden…")).toBeEnabled();
  });

  it("Status nicht abfragbar → keine Anmeldung (F-UI-39)", async () => {
    a.dropboxStatus.mockRejectedValue("kein Tauri");
    zeigen();
    await waitFor(() => expect(a.dropboxStatus).toHaveBeenCalled());
    await screen.findByLabelText("App-Schlüssel");
    expect(screen.queryByText("Bei Dropbox anmelden…")).toBeNull();
  });

  it("geänderter Schlüssel: Anmelden erst nach „Übernehmen“ (F-UI-89)", async () => {
    zeigen();
    const feld = await screen.findByLabelText("App-Schlüssel");
    expect(await screen.findByText("Bei Dropbox anmelden…")).toBeInTheDocument();
    await userEvent.clear(feld);
    await userEvent.type(feld, "eigen1");
    expect(screen.queryByText("Bei Dropbox anmelden…")).toBeNull();
    expect(screen.getByText("Anmelden nach „Übernehmen“.")).toBeInTheDocument();
    // „Standard“ setzt den mitgelieferten Schlüssel zurück
    await userEvent.click(screen.getByText("Standard"));
    expect(feld).toHaveValue(einstellungen.standardAppKey);
    expect(screen.queryByText("Standard")).toBeNull();
    expect(screen.getByText("Bei Dropbox anmelden…")).toBeInTheDocument();
  });

  it("leerer Schlüssel bei Dropbox-Regel → Hinweis (F-UI-89)", async () => {
    zeigen();
    const feld = await screen.findByLabelText("App-Schlüssel");
    expect(screen.queryByText(/Ohne App-Schlüssel/)).toBeNull();
    await userEvent.clear(feld);
    expect(screen.getByText(/Ohne App-Schlüssel werden Dropbox-Ordner wie lokale Ordner behandelt/)).toBeInTheDocument();
  });
});
