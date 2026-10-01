// @vitest-environment jsdom
import { useState } from "react";
import { afterEach, beforeEach, describe, it, expect, vi } from "vitest";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import MarketSettings from "./MarketSettings";
import MarketPrices from "./MarketPrices";
import PriceChart from "./PriceChart";
import {
  marketKeyStatus,
  saveMarketKey,
  removeMarketKey,
  refreshMarketPrices,
  type Snapshot,
  type Security,
  type PriceHistory,
} from "../api";
vi.mock("../api", () => ({
  getSnapshot: vi.fn(),
  openMarketSignup: vi.fn(),
  marketKeyStatus: vi.fn(),
  saveMarketKey: vi.fn(),
  removeMarketKey: vi.fn(),
  refreshMarketPrices: vi.fn(),
}));
const security: Security = {
  id: 1,
  ticker: "BEN",
  name: "Bank",
  currentPriceMicros: 10_600_000,
  priceUpdatedAt: "2026-10-01T06:00:00Z",
  priceSource: "eodhd",
  priceAsOf: "2026-09-30",
  marketFetchedAt: "2026-10-01T06:00:00Z",
  marketError: null,
};
const history: PriceHistory = {
  securityId: 1,
  fetchedAt: "2026-10-01T06:00:00Z",
  prices: [
    {
      sessionDate: "2026-09-29",
      closeMicros: 10_115_000,
      adjustedCloseMicros: 10_000_000,
    },
    {
      sessionDate: "2026-09-30",
      closeMicros: 10_600_000,
      adjustedCloseMicros: 10_500_000,
    },
  ],
};
function fixture(): Snapshot {
  return {
    profileId: "profile",
    displayName: "Tester",
    notes: [],
    market: {
      requestsToday: 1,
      dailyLimit: 20,
      refreshing: false,
      histories: [structuredClone(history)],
    },
    defaultBrokerageMicros: 0,
    currency: "AUD",
    primaryMarket: "ASX",
    startingCapitalMicros: 1_000_000_000,
    cashMicros: 1_000_000_000,
    createdAt: "2026-10-01T00:00:00Z",
    journal: [],
    trading: {
      securities: [structuredClone(security)],
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
}
function Harness({ initial = fixture() }: { initial?: Snapshot }) {
  const [s, setS] = useState(initial);
  return (
    <MarketPrices
      snapshot={s}
      security={s.trading.securities[0]}
      onChanged={setS}
    />
  );
}
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(marketKeyStatus).mockResolvedValue({
    configured: true,
    supported: true,
  });
  vi.mocked(saveMarketKey).mockResolvedValue(undefined);
  vi.mocked(removeMarketKey).mockResolvedValue(undefined);
});
afterEach(cleanup);
describe("EODHD setup and cached reference data", () => {
  it("saves a masked key, clears the input and removes it without clearing cached prices", async () => {
    const u = userEvent.setup();
    render(<MarketSettings market={fixture().market} />);
    await screen.findByText("API key saved · macOS Keychain");
    const input = screen.getByLabelText("EODHD API key") as HTMLInputElement;
    expect(input.type).toBe("password");
    await u.type(input, "example_key_123");
    await u.click(screen.getByRole("button", { name: "Replace API key" }));
    await screen.findByText(/API key saved in macOS Keychain/);
    expect(saveMarketKey).toHaveBeenCalledWith("example_key_123");
    expect(input.value).toBe("");
    await u.click(screen.getByRole("button", { name: "Remove API key" }));
    await screen.findByText("No API key saved");
    expect(removeMarketKey).toHaveBeenCalledOnce();
  });
  it("refreshes one stock and shows session date, retrieval time and chart", async () => {
    const u = userEvent.setup();
    const s = fixture();
    s.trading.securities[0].currentPriceMicros = 11_000_000;
    s.trading.securities[0].priceAsOf = "2026-10-01";
    vi.mocked(refreshMarketPrices).mockResolvedValue(s);
    render(<Harness />);
    await screen.findByRole("button", { name: "Refresh BEN price" });
    await u.click(screen.getByRole("button", { name: "Refresh BEN price" }));
    await screen.findByText(/Updated 1 stock from EODHD/);
    expect(refreshMarketPrices).toHaveBeenCalledWith([1]);
    expect(screen.getByText(/EODHD close · 2026-10-01/)).toBeTruthy();
    expect(
      screen.getByRole("img", { name: /unadjusted daily closing/ }),
    ).toBeTruthy();
  });
  it("retains cached chart and reference price on refresh failure", async () => {
    const u = userEvent.setup();
    vi.mocked(refreshMarketPrices).mockRejectedValue(
      "Offline. Cached prices kept.",
    );
    render(<Harness />);
    await u.click(
      await screen.findByRole("button", { name: "Refresh BEN price" }),
    );
    await screen.findByRole("alert");
    expect(screen.getByText(/EODHD close · 2026-09-30/)).toBeTruthy();
    expect(screen.getByRole("img")).toBeTruthy();
  });
  it("reports provider entitlement failure with the cached chart intact", async () => {
    const u = userEvent.setup();
    const s = fixture();
    s.trading.securities[0].marketError =
      "This key is not entitled to this ASX symbol.";
    vi.mocked(refreshMarketPrices).mockResolvedValue(s);
    render(<Harness />);
    await u.click(
      await screen.findByRole("button", { name: "Refresh BEN price" }),
    );
    await screen.findByText(/0 updated; 1 could not refresh/);
    expect(screen.getByRole("alert").textContent).toContain("not entitled");
    expect(screen.getByRole("img")).toBeTruthy();
  });
  it("disables paid requests when the local allowance is exhausted and prompts for a missing key", async () => {
    const s = fixture();
    s.market.requestsToday = 20;
    render(<Harness initial={s} />);
    await screen.findByRole("button", { name: "Refresh BEN price" });
    expect(
      (
        screen.getByRole("button", {
          name: "Refresh BEN price",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
    cleanup();
    vi.mocked(marketKeyStatus).mockResolvedValue({
      configured: false,
      supported: true,
    });
    render(<Harness />);
    await screen.findByText(/Add your free EODHD API key in Settings/);
    expect(
      (
        screen.getByRole("button", {
          name: "Refresh BEN price",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
    expect(refreshMarketPrices).not.toHaveBeenCalled();
  });
  it("lets users explore dates and explicitly switch adjusted series", async () => {
    const u = userEvent.setup();
    render(<PriceChart ticker="BEN" history={history} />);
    fireEvent.change(screen.getByRole("slider"), { target: { value: "0" } });
    expect(screen.getByRole("slider").getAttribute("aria-valuetext")).toContain(
      "2026-09-29: $10.115",
    );
    await u.click(screen.getByLabelText("Adjust for splits and dividends"));
    expect(screen.getByRole("img").getAttribute("aria-label")).toContain(
      "BEN adjusted",
    );
    expect(screen.getByRole("slider").getAttribute("aria-valuetext")).toContain(
      "$10",
    );
  });
});
