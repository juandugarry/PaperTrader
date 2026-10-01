// @vitest-environment jsdom
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import TradingWorkspace from "./TradingWorkspace";
import { createSecurity, setPrice, executeTrade, type Snapshot } from "../api";
vi.mock("../api", () => ({
  createSecurity: vi.fn(),
  setPrice: vi.fn(),
  executeTrade: vi.fn(),
}));
const security = {
  id: 1,
  ticker: "BEN",
  name: "Bendigo and Adelaide Bank",
  currentPriceMicros: null,
  priceUpdatedAt: null,
};
function fresh(): Snapshot {
  return {
    displayName: "Alex",
    currency: "AUD",
    primaryMarket: "ASX",
    defaultBrokerageMicros: 3_000_000,
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
}
function withSecurity() {
  const s = fresh();
  s.trading.securities = [security];
  return s;
}
function bought() {
  const s = withSecurity();
  s.cashMicros = 754_240_000;
  s.trading.capitalInvestedMicros = 245_760_000;
  s.trading.brokeragePaidMicros = 3_000_000;
  s.trading.portfolioValueMicros = null;
  s.trading.unrealisedPnlMicros = null;
  s.trading.totalReturnMicros = null;
  s.trading.positions = [
    {
      securityId: 1,
      ticker: "BEN",
      name: security.name,
      quantity: 24,
      averageEntryMicros: 10_115_000,
      costBasisMicros: 245_760_000,
      currentPriceMicros: null,
      priceUpdatedAt: null,
      valueMicros: null,
      unrealisedPnlMicros: null,
    },
  ];
  s.trading.executions = [
    {
      id: 1,
      securityId: 1,
      ticker: "BEN",
      name: security.name,
      side: "BUY",
      quantity: 24,
      priceMicros: 10_115_000,
      brokerageMicros: 3_000_000,
      notionalMicros: 242_760_000,
      cashDeltaMicros: -245_760_000,
      createdAt: "2026-10-01T01:00:00Z",
    },
  ];
  return s;
}
function Harness({ initial }: { initial: Snapshot }) {
  const [snapshot, setSnapshot] = useState(initial);
  return <TradingWorkspace snapshot={snapshot} onChanged={setSnapshot} />;
}
beforeEach(() => vi.resetAllMocks());
afterEach(cleanup);
async function prepareBuy(user: ReturnType<typeof userEvent.setup>) {
  await user.type(screen.getByLabelText("Quantity (whole shares)"), "24");
  await user.type(screen.getByLabelText("Fill price (AUD)"), "10.115");
  await user.click(
    screen.getByRole("button", { name: "Review simulated buy" }),
  );
}
describe("manual trading workflow", () => {
  it("starts empty and creates/searches a security without importing any holdings", async () => {
    vi.mocked(createSecurity).mockResolvedValue(withSecurity());
    const user = userEvent.setup();
    render(<Harness initial={fresh()} />);
    await user.click(screen.getByRole("button", { name: "Add security" }));
    await user.type(screen.getByLabelText("ASX ticker"), "ben");
    await user.type(screen.getByLabelText("Security name"), security.name);
    await user.click(screen.getByRole("button", { name: "Save security" }));
    await screen.findByRole("heading", { name: `BEN · ${security.name}` });
    expect(createSecurity).toHaveBeenCalledWith({
      ticker: "BEN",
      name: security.name,
    });
    await user.type(
      screen.getByLabelText("Search local securities"),
      "not-a-ticker",
    );
    expect(screen.getByText("No matching securities.")).toBeTruthy();
    expect(executeTrade).not.toHaveBeenCalled();
  });
  it("reviews exact buy outlay before executing and shows updated cash and holding", async () => {
    vi.mocked(executeTrade).mockResolvedValue(bought());
    const user = userEvent.setup();
    render(<Harness initial={withSecurity()} />);
    await prepareBuy(user);
    expect(screen.getByText("$245.76")).toBeTruthy();
    expect(screen.getByText("$754.24")).toBeTruthy();
    expect(executeTrade).not.toHaveBeenCalled();
    await user.click(
      screen.getByRole("button", { name: "Confirm simulated fill" }),
    );
    await screen.findByRole("status");
    expect(executeTrade).toHaveBeenCalledWith(
      expect.objectContaining({
        requestId: expect.any(String),
        securityId: 1,
        side: "BUY",
        quantity: 24,
        priceMicros: 10_115_000,
        brokerageMicros: 3_000_000,
      }),
    );
    expect(
      screen.getByText("24 shares held · $245.76 cost basis"),
    ).toBeTruthy();
    expect(screen.getByText("1 fills · immutable")).toBeTruthy();
  });
  it("prevents overselling and purchases exceeding cash", async () => {
    const user = userEvent.setup();
    render(<Harness initial={withSecurity()} />);
    await user.click(screen.getByRole("button", { name: "SELL" }));
    await user.type(screen.getByLabelText("Quantity (whole shares)"), "1");
    await user.type(screen.getByLabelText("Fill price (AUD)"), "10");
    await user.click(
      screen.getByRole("button", { name: "Review simulated sell" }),
    );
    expect((await screen.findByRole("alert")).textContent).toMatch(
      /more shares/,
    );
    await user.click(screen.getByRole("button", { name: "BUY" }));
    await user.clear(screen.getByLabelText("Quantity (whole shares)"));
    await user.type(screen.getByLabelText("Quantity (whole shares)"), "100");
    await user.click(
      screen.getByRole("button", { name: "Review simulated buy" }),
    );
    expect((await screen.findByRole("alert")).textContent).toMatch(
      /Insufficient/,
    );
    expect(executeTrade).not.toHaveBeenCalled();
  });
  it("retries a failed confirmation with the same idempotency key", async () => {
    vi.mocked(executeTrade)
      .mockRejectedValueOnce("Database busy")
      .mockResolvedValueOnce(bought());
    const user = userEvent.setup();
    render(<Harness initial={withSecurity()} />);
    await prepareBuy(user);
    await user.click(
      screen.getByRole("button", { name: "Confirm simulated fill" }),
    );
    expect((await screen.findByRole("alert")).textContent).toBe(
      "Database busy",
    );
    await user.click(
      screen.getByRole("button", { name: "Confirm simulated fill" }),
    );
    await screen.findByRole("status");
    expect(vi.mocked(executeTrade).mock.calls[0][0].requestId).toBe(
      vi.mocked(executeTrade).mock.calls[1][0].requestId,
    );
  });
  it("records a partial sell with overridden brokerage and refreshes the position", async () => {
    const after = bought();
    after.cashMicros = 879_440_000;
    after.trading.positions[0].quantity = 12;
    after.trading.positions[0].costBasisMicros = 122_880_000;
    after.trading.realisedPnlMicros = 2_320_000;
    vi.mocked(executeTrade).mockResolvedValue(after);
    const user = userEvent.setup();
    render(<Harness initial={bought()} />);
    await user.click(screen.getByRole("button", { name: "SELL" }));
    await user.type(screen.getByLabelText("Quantity (whole shares)"), "12");
    await user.type(screen.getByLabelText("Fill price (AUD)"), "10.60");
    await user.clear(screen.getByLabelText("Brokerage (AUD)"));
    await user.type(screen.getByLabelText("Brokerage (AUD)"), "2");
    await user.click(
      screen.getByRole("button", { name: "Review simulated sell" }),
    );
    expect(screen.getByText("$125.20")).toBeTruthy();
    await user.click(
      screen.getByRole("button", { name: "Confirm simulated fill" }),
    );
    await screen.findByRole("status");
    expect(executeTrade).toHaveBeenCalledWith(
      expect.objectContaining({
        side: "SELL",
        quantity: 12,
        brokerageMicros: 2_000_000,
      }),
    );
    expect(
      screen.getByText("12 shares held · $122.88 cost basis"),
    ).toBeTruthy();
  });
  it("saves an exact manual price and exposes its timestamp without executing a fill", async () => {
    const after = withSecurity();
    after.trading.securities = [
      {
        ...security,
        currentPriceMicros: 10_115_000,
        priceUpdatedAt: "2026-10-01T02:00:00Z",
      },
    ];
    vi.mocked(setPrice).mockResolvedValue(after);
    const user = userEvent.setup();
    render(<Harness initial={withSecurity()} />);
    await user.type(
      screen.getByLabelText("Manual current price (AUD)"),
      "10.115",
    );
    await user.click(screen.getByRole("button", { name: "Update price" }));
    await screen.findByText(/Manually updated/);
    expect(setPrice).toHaveBeenCalledWith({
      securityId: 1,
      priceMicros: 10_115_000,
    });
    expect(executeTrade).not.toHaveBeenCalled();
  });
  it("captures a journal plan in the reviewed buy and submits it with the fill", async () => {
    vi.mocked(executeTrade).mockResolvedValue(bought());
    const user = userEvent.setup();
    render(<Harness initial={withSecurity()} />);
    await user.click(screen.getByText("Entry plan & notes"));
    await user.type(
      screen.getByLabelText("Trading thesis"),
      "Support may hold",
    );
    await user.type(
      screen.getByLabelText("Entry trigger"),
      "Bounce from support",
    );
    await user.type(screen.getByLabelText("Target price (AUD)"), "10.60");
    await user.type(
      screen.getByLabelText("Stop / invalidation price (AUD)"),
      "9.95",
    );
    await user.type(screen.getByLabelText("Planned risk (AUD)"), "10");
    await prepareBuy(user);
    expect(executeTrade).not.toHaveBeenCalled();
    await user.click(
      screen.getByRole("button", { name: "Confirm simulated fill" }),
    );
    await screen.findByRole("status");
    expect(executeTrade).toHaveBeenCalledWith(
      expect.objectContaining({
        journal: expect.objectContaining({
          thesis: "Support may hold",
          entryTrigger: "Bounce from support",
          targetMicros: 10_600_000,
          stopMicros: 9_950_000,
          plannedRiskMicros: 10_000_000,
        }),
      }),
    );
  });
});
