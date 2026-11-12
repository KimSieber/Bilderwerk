// Fenster „Über Bilderwerk View“ (v0.3)
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/app", () => ({ getVersion: vi.fn().mockResolvedValue("0.3.0") }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn().mockResolvedValue(undefined) }));
import { openUrl } from "@tauri-apps/plugin-opener";
import Ueber, { EMAIL, LIZENZ_URL, WEBSEITE } from "./Ueber";

describe("Über Bilderwerk View", () => {
  it("Version, Copyright, Lizenz mit Ausnahme der Icons (F-UI-91)", async () => {
    render(<Ueber />);
    expect(await screen.findByText("Version 0.3.0")).toBeInTheDocument();
    expect(screen.getByText(/© 2026 Kim Sieber/)).toBeInTheDocument();
    expect(screen.getByText("PolyForm Noncommercial 1.0.0")).toBeInTheDocument();
    expect(screen.getByText(/ausgenommen Icons/)).toBeInTheDocument();
  });

  it("Webseite, E-Mail und Lizenz öffnen sich im Standardprogramm (F-UI-91)", async () => {
    render(<Ueber />);
    await userEvent.click(screen.getByText("bilderwerk.sieber-bw.de"));
    await userEvent.click(screen.getByText(EMAIL));
    await userEvent.click(screen.getByText("PolyForm Noncommercial 1.0.0"));
    expect(vi.mocked(openUrl).mock.calls.map((c) => c[0])).toEqual([WEBSEITE, `mailto:${EMAIL}`, LIZENZ_URL]);
  });
});
