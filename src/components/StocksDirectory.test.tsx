// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import StocksDirectory from "./StocksDirectory";
import {
  getAsxDirectory,
  refreshAsxDirectory,
  createSecurity,
  getSnapshot,
  type Snapshot,
} from "../api";
vi.mock("../api", () => ({
  getAsxDirectory: vi.fn(),
  refreshAsxDirectory: vi.fn(),
  createSecurity: vi.fn(),
  getSnapshot: vi.fn(),
}));
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
const snapshot = {
  market: { dailyLimit: 20, requestsToday: 0, refreshing: false },
  trading: { securities: [] },
} as unknown as Snapshot;
const entries = [
  { ticker: "BHP", name: "BHP Group", kind: "Common Stock", currency: "AUD" },
  {
    ticker: "VAS",
    name: "Vanguard Australian Shares",
    kind: "ETF",
    currency: "AUD",
  },
];
it("searches cached names and tickers, filters types, and adds without fetching prices", async () => {
  vi.mocked(getAsxDirectory).mockResolvedValue({
    entries,
    fetchedAt: "2026-10-01T00:00:00Z",
  });
  vi.mocked(createSecurity).mockResolvedValue(snapshot);
  const onChanged = vi.fn();
  render(<StocksDirectory snapshot={snapshot} onChanged={onChanged} />);
  await screen.findByText("BHP Group");
  fireEvent.change(screen.getByLabelText("Search company or ticker"), {
    target: { value: "vanguard" },
  });
  expect(screen.queryByText("BHP Group")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Add VAS to Trade" }));
  await screen.findByText(/VAS added/);
  expect(createSecurity).toHaveBeenCalledWith({
    ticker: "VAS",
    name: "Vanguard Australian Shares",
  });
  expect(refreshAsxDirectory).not.toHaveBeenCalled();
  fireEvent.change(screen.getByLabelText("Search company or ticker"), {
    target: { value: "" },
  });
  fireEvent.change(screen.getByLabelText("Listing type"), {
    target: { value: "Common Stock" },
  });
  expect(screen.queryByText("Vanguard Australian Shares")).toBeNull();
  expect(screen.getByText("BHP Group")).toBeTruthy();
});
it("retains the offline list after an update failure and reloads request usage", async () => {
  vi.mocked(getAsxDirectory).mockResolvedValue({
    entries,
    fetchedAt: "2026-10-01T00:00:00Z",
  });
  vi.mocked(refreshAsxDirectory).mockRejectedValue("Provider unavailable");
  vi.mocked(getSnapshot).mockResolvedValue(snapshot);
  render(<StocksDirectory snapshot={snapshot} onChanged={vi.fn()} />);
  await screen.findByText("BHP Group");
  fireEvent.click(screen.getByRole("button", { name: "Update directory" }));
  expect(await screen.findByRole("alert")).toHaveProperty(
    "textContent",
    "Provider unavailable",
  );
  expect(screen.getByText("BHP Group")).toBeTruthy();
});
