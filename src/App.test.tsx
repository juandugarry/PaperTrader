// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";
import {
  createProfile,
  getSnapshot,
  resetProfile,
  marketKeyStatus,
  type Snapshot,
} from "./api";
vi.mock("./api", () => ({
  desktopAvailable: true,
  openMarketSignup: vi.fn(),
  marketKeyStatus: vi
    .fn()
    .mockResolvedValue({ configured: false, supported: true }),
  saveMarketKey: vi.fn(),
  removeMarketKey: vi.fn(),
  refreshMarketPrices: vi.fn(),
  getSnapshot: vi.fn(),
  createProfile: vi.fn(),
  resetProfile: vi.fn(),
  saveNote: vi.fn(),
  deleteNote: vi.fn(),
  saveJournal: vi.fn(),
  createSecurity: vi.fn(),
  setPrice: vi.fn(),
  executeTrade: vi.fn(),
}));
const portfolio: Snapshot = {
  displayName: "Alex",
  profileId: "test-profile",
  notes: [],
  market: {
    requestsToday: 0,
    dailyLimit: 20,
    refreshing: false,
    histories: [],
  },
  defaultBrokerageMicros: 3_000_000,
  currency: "AUD",
  primaryMarket: "ASX",
  startingCapitalMicros: 1_000_000_000,
  cashMicros: 1_000_000_000,
  createdAt: "2026-10-01T00:00:00Z",
  journal: [],
  trading: {
    securities: [],
    positions: [],
    executions: [],
    capitalInvestedMicros: 0,
    realisedPnlMicros: 0,
    brokeragePaidMicros: 0,
    unrealisedPnlMicros: 0,
    portfolioValueMicros: 1_000_000_000,
    totalReturnMicros: 0,
  },
};
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(marketKeyStatus).mockResolvedValue({
    configured: false,
    supported: true,
  });
  vi.mocked(getSnapshot).mockResolvedValue(null);
});
afterEach(cleanup);
describe("Profile and navigation through the IPC boundary", () => {
  it("creates a virtual portfolio with exact monetary values and navigates all areas", async () => {
    vi.mocked(createProfile).mockResolvedValue(portfolio);
    const user = userEvent.setup();
    render(<App />);
    await user.type(await screen.findByLabelText("Trader name"), "Alex");
    await user.click(
      screen.getByRole("button", { name: /Create my virtual portfolio/ }),
    );
    await screen.findByRole("heading", { name: "Your portfolio" });
    expect(createProfile).toHaveBeenCalledWith({
      displayName: "Alex",
      startingCapitalMicros: 1_000_000_000,
      defaultBrokerageMicros: 3_000_000,
    });
    expect(screen.getAllByText("$1,000.00").length).toBeGreaterThan(0);
    await user.click(screen.getByRole("button", { name: /Journal/ }));
    expect(
      screen.getByRole("heading", { name: "Trading journal" }),
    ).toBeTruthy();
    await user.click(screen.getByRole("button", { name: /Learn/ }));
    await user.click(
      screen.getByRole("button", { name: /Brokerage & transaction costs/ }),
    );
    expect(
      screen.getByRole("heading", { name: "Brokerage & transaction costs" }),
    ).toBeTruthy();
    await user.click(screen.getByRole("button", { name: /Settings/ }));
    expect(screen.getByText("$3.00 per transaction")).toBeTruthy();
  });
  it("rejects invalid money before invoking persistence", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.type(await screen.findByLabelText("Trader name"), "Alex");
    const capital = screen.getByLabelText("Starting capital (AUD)");
    await user.clear(capital);
    await user.type(capital, "-100");
    await user.click(
      screen.getByRole("button", { name: /Create my virtual portfolio/ }),
    );
    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(createProfile).not.toHaveBeenCalled();
  });
  it("shows persistence errors and permits a retry", async () => {
    vi.mocked(createProfile)
      .mockRejectedValueOnce("Database unavailable")
      .mockResolvedValueOnce(portfolio);
    const user = userEvent.setup();
    render(<App />);
    await user.type(await screen.findByLabelText("Trader name"), "Alex");
    await user.click(
      screen.getByRole("button", { name: /Create my virtual portfolio/ }),
    );
    expect((await screen.findByRole("alert")).textContent).toBe(
      "Database unavailable",
    );
    await user.click(
      screen.getByRole("button", { name: /Create my virtual portfolio/ }),
    );
    await screen.findByRole("heading", { name: "Your portfolio" });
  });
  it("loads existing profile instead of presenting setup again", async () => {
    vi.mocked(getSnapshot).mockResolvedValue(portfolio);
    render(<App />);
    await screen.findByRole("heading", { name: "Your portfolio" });
    expect(screen.queryByLabelText("Trader name")).toBeNull();
    expect(createProfile).not.toHaveBeenCalled();
  });
  it("reports startup errors and retries loading", async () => {
    vi.mocked(getSnapshot)
      .mockRejectedValueOnce("Unable to read database")
      .mockResolvedValueOnce(portfolio);
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", {
      name: "Unable to open your portfolio",
    });
    await user.click(screen.getByRole("button", { name: "Try again" }));
    await waitFor(() =>
      expect(
        screen.getByRole("heading", { name: "Your portfolio" }),
      ).toBeTruthy(),
    );
  });
  it("returns to first-launch setup after an explicitly confirmed profile reset", async () => {
    vi.mocked(getSnapshot).mockResolvedValue(portfolio);
    vi.mocked(resetProfile).mockResolvedValue();
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: "Your portfolio" });
    await user.click(screen.getByRole("button", { name: /Settings/ }));
    await user.click(screen.getByRole("button", { name: "Reset profile…" }));
    await user.type(screen.getByLabelText("Type RESET to confirm"), "RESET");
    await user.click(
      screen.getByRole("button", { name: "Permanently reset profile" }),
    );
    await screen.findByLabelText("Trader name");
    expect(resetProfile).toHaveBeenCalledWith("RESET");
  });
  it("keeps an unsaved notepad draft when changing the main page until it is discarded", async () => {
    vi.mocked(getSnapshot).mockResolvedValue(portfolio);
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: "Your portfolio" });
    await user.click(screen.getByRole("button", { name: /Journal/ }));
    await user.click(screen.getByRole("button", { name: "Notepad" }));
    await user.click(screen.getByRole("button", { name: "New note" }));
    await user.type(screen.getByLabelText("Note title"), "Unsaved question");
    await user.click(screen.getByRole("button", { name: /Settings/ }));
    expect(
      (screen.getByLabelText("Note title") as HTMLInputElement).value,
    ).toBe("Unsaved question");
    await user.click(screen.getByRole("button", { name: "Stay in notepad" }));
    expect(screen.getByRole("heading", { name: "Your notepad" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: /Settings/ }));
    await user.click(
      screen.getByRole("button", { name: "Discard note changes and continue" }),
    );
    expect(
      screen.getByRole("heading", { name: "Your local workspace" }),
    ).toBeTruthy();
  });
});
