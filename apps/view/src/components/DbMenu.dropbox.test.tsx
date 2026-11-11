import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({
  ...(await orig<typeof import("../api")>()),
  api: {
    dropboxStatus: vi.fn(),
    dropboxAnmelden: vi.fn(),
    dropboxAbmelden: vi.fn(),
    cacheInfo: vi.fn().mockRejectedValue("kein Tauri"),
  },
}));
vi.mock("../vorschau", () => ({ nurOnlineNeuLaden: vi.fn(), alleNeuLaden: vi.fn() }));
import { api } from "../api";
import { nurOnlineNeuLaden } from "../vorschau";
import { info } from "../test/daten";
import DbMenu from "./DbMenu";

const a = vi.mocked(api);
beforeEach(() => {
  a.dropboxStatus.mockReset().mockResolvedValue({ eingerichtet: true, angemeldet: false });
  a.dropboxAnmelden.mockReset().mockResolvedValue({ eingerichtet: true, angemeldet: true });
  a.dropboxAbmelden.mockReset().mockResolvedValue({ eingerichtet: true, angemeldet: false });
  vi.mocked(nurOnlineNeuLaden).mockReset();
});
const oeffnen = () => userEvent.click(screen.getByTitle("Datenbank"));

describe("DbMenu – Dropbox", () => {
  it("ohne App-Schlüssel kein Dropbox-Bereich (F-UI-35)", async () => {
    a.dropboxStatus.mockResolvedValue({ eingerichtet: false, angemeldet: false });
    render(<DbMenu info={info} onWechseln={vi.fn()} />);
    await oeffnen();
    await waitFor(() => expect(a.dropboxStatus).toHaveBeenCalled());
    expect(screen.queryByText("Dropbox", { exact: true })).toBeNull();
    expect(screen.queryByText("Bei Dropbox anmelden…")).toBeNull();
  });

  it("Status nicht angemeldet → Anmelden lädt Wolken neu (F-UI-36)", async () => {
    let fertig!: (s: { eingerichtet: boolean; angemeldet: boolean }) => void;
    a.dropboxAnmelden.mockImplementation(() => new Promise((r) => (fertig = r)));
    render(<DbMenu info={info} onWechseln={vi.fn()} />);
    await oeffnen();
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
    render(<DbMenu info={info} onWechseln={vi.fn()} />);
    await oeffnen();
    await userEvent.click(await screen.findByText("Von Dropbox abmelden"));
    expect(a.dropboxAbmelden).toHaveBeenCalledOnce();
    expect(await screen.findByText("nicht angemeldet")).toBeInTheDocument();
    expect(nurOnlineNeuLaden).not.toHaveBeenCalled();
  });

  it("Fehler bei der Anmeldung wird angezeigt (F-UI-38)", async () => {
    a.dropboxAnmelden.mockRejectedValue("Zeit für die Anmeldung abgelaufen.");
    render(<DbMenu info={info} onWechseln={vi.fn()} />);
    await oeffnen();
    await userEvent.click(await screen.findByText("Bei Dropbox anmelden…"));
    expect(await screen.findByText("Zeit für die Anmeldung abgelaufen.")).toBeInTheDocument();
    expect(screen.getByText("Bei Dropbox anmelden…")).toBeEnabled();
  });

  it("Status nicht abfragbar → Bereich bleibt weg (F-UI-39)", async () => {
    a.dropboxStatus.mockRejectedValue("kein Tauri");
    render(<DbMenu info={info} onWechseln={vi.fn()} />);
    await oeffnen();
    await waitFor(() => expect(a.dropboxStatus).toHaveBeenCalled());
    expect(screen.queryByText("Dropbox", { exact: true })).toBeNull();
    expect(screen.queryByText("Bei Dropbox anmelden…")).toBeNull();
  });
});
