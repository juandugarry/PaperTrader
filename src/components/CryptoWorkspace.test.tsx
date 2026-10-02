// @vitest-environment jsdom
import { afterEach, beforeEach, it, expect, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import CryptoWorkspace from "./CryptoWorkspace";
import CryptoTradeTicket from "./CryptoTradeTicket";
import {
  getCryptoMarket,
  refreshCryptoMarket,
  setupCryptoWallet,
  executeCryptoTrade,
  type Snapshot,
  type CryptoMarket,
} from "../api";
vi.mock("./useCryptoLive", () => ({ default: () => "Live feed connected" }));
vi.mock("../api", () => ({
  getCryptoMarket: vi.fn(),
  getSnapshot: vi.fn(),
  refreshCryptoMarket: vi.fn(),
  setupCryptoWallet: vi.fn(),
  executeCryptoTrade: vi.fn(),
  depositVirtualFunds: vi.fn(),
}));
const market: CryptoMarket = {
  assets: [
    {
      pair: "XXBTZUSD",
      symbol: "BTC",
      name: "Bitcoin",
      wsSymbol: "BTC/USD",
      quote: "USD",
    },
    {
      pair: "XETHZUSD",
      symbol: "ETH",
      name: "Ethereum",
      wsSymbol: "ETH/USD",
      quote: "USD",
    },
  ],
  quotes: [
    {
      pair: "XXBTZUSD",
      pricePicos: "100000000000000000",
      usdPricePicos: "65000000000000000",
      fetchedAt: "2026-10-02T00:00:00Z",
      source: "rest",
    },
  ],
  charts: [],
  catalogFetchedAt: "2026-10-02T00:00:00Z",
  fxFetchedAt: "2026-10-02T00:00:00Z",
  lastError: null,
  refreshing: false,
};
function fixture(): Snapshot {
  return {
    profileId: "test",
    displayName: "Test",
    notes: [],
    market: {
      requestsToday: 20,
      dailyLimit: 20,
      refreshing: false,
      histories: [],
    },
    defaultBrokerageMicros: 0,
    currency: "AUD",
    primaryMarket: "ASX",
    startingCapitalMicros: 1000000000,
    cashMicros: 1000000000,
    createdAt: "2026-10-02T00:00:00Z",
    trading: {
      securities: [],
      positions: [],
      executions: [],
      capitalInvestedMicros: 0,
      realisedPnlMicros: 0,
      brokeragePaidMicros: 0,
      unrealisedPnlMicros: 0,
      portfolioValueMicros: 1000000000,
      totalReturnMicros: 0,
    },
    journal: [],
    crypto: {
      wallet: null,
      positions: [],
      executions: [],
      assetValueMicros: 0,
    },
  };
}
function funded(): Snapshot {
  const s = fixture();
  s.crypto!.wallet = {
    mode: "separate",
    cashMicros: 500000000,
    contributionsMicros: 500000000,
    portfolioValueMicros: 500000000,
    totalReturnMicros: 0,
    realisedPnlMicros: 0,
    feeMicros: 0,
  };
  return s;
}
function Harness() {
  const [snapshot, setSnapshot] = useState(fixture());
  return <CryptoWorkspace snapshot={snapshot} onChanged={setSnapshot} />;
}
beforeEach(() => {
  vi.mocked(getCryptoMarket).mockResolvedValue(structuredClone(market));
});
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
it("defaults to separate fresh funds and confirms wallet creation explicitly", async () => {
  vi.mocked(setupCryptoWallet).mockResolvedValue(funded());
  render(<Harness />);
  const u = userEvent.setup();
  await screen.findByText("Bitcoin");
  expect(
    (screen.getByLabelText("Crypto funding") as HTMLSelectElement).value,
  ).toBe("separate");
  await u.clear(screen.getByLabelText("Starting virtual crypto funds (AUD)"));
  await u.type(
    screen.getByLabelText("Starting virtual crypto funds (AUD)"),
    "500.00",
  );
  await u.click(screen.getByRole("button", { name: "Review wallet setup" }));
  expect(setupCryptoWallet).not.toHaveBeenCalled();
  await u.click(
    screen.getByRole("button", { name: "Confirm virtual wallet setup" }),
  );
  await screen.findByText("Crypto cash");
  expect(setupCryptoWallet).toHaveBeenCalledWith(
    expect.objectContaining({
      mode: "separate",
      startingFundsMicros: 500000000,
    }),
  );
  expect(refreshCryptoMarket).not.toHaveBeenCalled();
});
it("offers shared funds, searches the catalogue, and refreshes crypto independently of exhausted ASX allowance", async () => {
  vi.mocked(refreshCryptoMarket).mockResolvedValue({
    market,
    snapshot: fixture(),
  });
  render(<Harness />);
  const u = userEvent.setup();
  await screen.findByText("Bitcoin");
  await u.selectOptions(screen.getByLabelText("Crypto funding"), "shared");
  expect(
    screen.queryByLabelText("Starting virtual crypto funds (AUD)"),
  ).toBeNull();
  await u.type(
    screen.getByLabelText("Search crypto name or symbol"),
    "ethereum",
  );
  expect(screen.queryByRole("button", { name: "Open BTC crypto" })).toBeNull();
  expect(screen.getByRole("button", { name: "Open ETH crypto" })).toBeTruthy();
  await u.click(
    screen.getByRole("button", { name: "Refresh all crypto prices" }),
  );
  expect(refreshCryptoMarket).toHaveBeenCalledWith("quotes", null);
});
it("freezes a reviewed fractional crypto price while live quotes change", async () => {
  const snapshot = funded();
  vi.mocked(executeCryptoTrade).mockResolvedValue(snapshot);
  const onChanged = vi.fn();
  const { rerender } = render(
    <CryptoTradeTicket
      asset={market.assets[0]}
      quote={market.quotes[0]}
      snapshot={snapshot}
      onChanged={onChanged}
    />,
  );
  const u = userEvent.setup();
  await u.type(screen.getByLabelText("Crypto quantity"), "0.001");
  await u.clear(screen.getByLabelText("Virtual trading fee (AUD)"));
  await u.type(screen.getByLabelText("Virtual trading fee (AUD)"), "1.00");
  await u.click(
    screen.getByRole("button", { name: "Review simulated crypto trade" }),
  );
  expect(executeCryptoTrade).not.toHaveBeenCalled();
  rerender(
    <CryptoTradeTicket
      asset={market.assets[0]}
      quote={{ ...market.quotes[0], pricePicos: "200000000000000000" }}
      snapshot={snapshot}
      onChanged={onChanged}
    />,
  );
  await u.click(
    screen.getByRole("button", { name: "Confirm simulated crypto trade" }),
  );
  expect(executeCryptoTrade).toHaveBeenCalledWith(
    expect.objectContaining({
      side: "BUY",
      quantityAtoms: "100000",
      pricePicos: "100000000000000000",
      feeMicros: 1000000,
    }),
  );
});
