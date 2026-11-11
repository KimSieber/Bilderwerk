import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({
  ...(await orig<typeof import("../api")>()),
  api: {
    dbNeuLaden: vi.fn(),
    dropboxStatus: vi.fn().mockRejectedValue("x"),
    cacheInfo: vi.fn().mockRejectedValue("x"),
  },
}));
import { api } from "../api";
import { info } from "../test/daten";
import DbMenu from "./DbMenu";

describe("DbMenu Neu laden (F-UI-81)", () => {
  it("ruft db_neu_laden; Fehler wird angezeigt", async () => {
    const a = vi.mocked(api);
    a.dbNeuLaden.mockResolvedValueOnce(info).mockRejectedValueOnce("Datei nicht gefunden: /x.db");
    render(<DbMenu info={info} onWechseln={vi.fn()} />);
    await userEvent.click(screen.getByTitle("Datenbank"));
    await userEvent.click(screen.getByText("Neu laden"));
    expect(a.dbNeuLaden).toHaveBeenCalledOnce();
    expect(screen.queryByText(/nicht gefunden/)).toBeNull();
    await userEvent.click(screen.getByText("Neu laden"));
    expect(await screen.findByText("Datei nicht gefunden: /x.db")).toBeInTheDocument();
  });
});
