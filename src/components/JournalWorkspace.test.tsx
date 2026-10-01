// @vitest-environment jsdom
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import JournalWorkspace from "./JournalWorkspace";
import {
  saveJournal,
  getSnapshot,
  type Snapshot,
  type JournalContent,
  type JournalTrade,
  type Execution,
} from "../api";
vi.mock("../api", () => ({ saveJournal: vi.fn(), getSnapshot: vi.fn() }));
const blank = (): JournalContent => ({
  thesis: "",
  entryTrigger: "",
  targetMicros: null,
  stopMicros: null,
  plannedRiskMicros: null,
  notes: "",
  exitReason: "",
  followedPlan: null,
  wentWell: "",
  wentPoorly: "",
  wouldChange: "",
});
const buy: Execution = {
  id: 1,
  securityId: 1,
  ticker: "BEN",
  name: "Bendigo and Adelaide Bank",
  side: "BUY",
  quantity: 24,
  priceMicros: 10_115_000,
  brokerageMicros: 3_000_000,
  notionalMicros: 242_760_000,
  cashDeltaMicros: -245_760_000,
  createdAt: "2026-10-01T00:00:00Z",
};
function fixture(): Snapshot {
  const sell: Execution = {
    ...buy,
    id: 2,
    side: "SELL",
    priceMicros: 10_600_000,
    notionalMicros: 254_400_000,
    cashDeltaMicros: 251_400_000,
    createdAt: "2026-10-02T00:00:00Z",
  };
  const trade: JournalTrade = {
    id: 1,
    securityId: 1,
    ticker: "BEN",
    name: buy.name,
    openedAt: buy.createdAt,
    closedAt: sell.createdAt,
    quantityBought: 24,
    quantitySold: 24,
    quantityOpen: 0,
    entryCostMicros: 245_760_000,
    brokeragePaidMicros: 6_000_000,
    grossPnlMicros: 11_640_000,
    netPnlMicros: 5_640_000,
    realisedCostsMicros: 6_000_000,
    realisedBasisMicros: 245_760_000,
    fills: [
      {
        execution: buy,
        capturedWithFill: true,
        revisions: [
          {
            version: 1,
            content: {
              ...blank(),
              thesis: "Original support thesis",
              entryTrigger: "Bounce from support",
              targetMicros: 10_600_000,
              stopMicros: 9_950_000,
              plannedRiskMicros: 10_000_000,
            },
            createdAt: buy.createdAt,
          },
        ],
        grossPnlMicros: null,
        netPnlMicros: null,
        transactionCostsMicros: null,
        releasedCostMicros: null,
      },
      {
        execution: sell,
        capturedWithFill: true,
        revisions: [
          { version: 1, content: blank(), createdAt: sell.createdAt },
        ],
        grossPnlMicros: 11_640_000,
        netPnlMicros: 5_640_000,
        transactionCostsMicros: 6_000_000,
        releasedCostMicros: 245_760_000,
      },
    ],
  };
  return {
    displayName: "Alex",
    defaultBrokerageMicros: 3_000_000,
    currency: "AUD",
    primaryMarket: "ASX",
    startingCapitalMicros: 1_000_000_000,
    cashMicros: 1_005_640_000,
    createdAt: buy.createdAt,
    trading: {
      securities: [],
      positions: [],
      executions: [buy, sell],
      capitalInvestedMicros: 0,
      realisedPnlMicros: 5_640_000,
      brokeragePaidMicros: 6_000_000,
      unrealisedPnlMicros: 0,
      portfolioValueMicros: 1_005_640_000,
      totalReturnMicros: 5_640_000,
    },
    journal: [trade],
  };
}
function Harness({ initial = fixture() }: { initial?: Snapshot }) {
  const [snapshot, setSnapshot] = useState(initial);
  return <JournalWorkspace snapshot={snapshot} onChanged={setSnapshot} />;
}
function amended(thesis: string) {
  const s = fixture();
  s.journal[0].fills[0].revisions.push({
    version: 2,
    content: { ...s.journal[0].fills[0].revisions[0].content, thesis },
    createdAt: "2026-10-03T00:00:00Z",
  });
  return s;
}
beforeEach(() => vi.resetAllMocks());
afterEach(cleanup);
describe("linked trading journal", () => {
  it("shows plans, immutable fill facts and fee-adjusted outcome with status filtering", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    expect(screen.getByText("Original support thesis")).toBeTruthy();
    expect(screen.getByText("Net $5.64")).toBeTruthy();
    expect(
      screen.getByText("Closed-trade return: 2.29%", { exact: false }),
    ).toBeTruthy();
    await user.selectOptions(screen.getByLabelText("Trade status"), "open");
    expect(
      screen.getByRole("heading", { name: "No matching trades" }),
    ).toBeTruthy();
    await user.selectOptions(screen.getByLabelText("Trade status"), "closed");
    expect(
      screen.getByRole("heading", { name: "BEN · Bendigo and Adelaide Bank" }),
    ).toBeTruthy();
    await user.type(screen.getByLabelText("Search journal"), "XYZ");
    expect(
      screen.getByRole("heading", { name: "No matching trades" }),
    ).toBeTruthy();
  });
  it("edits commentary with optimistic versioning and displays the original revision", async () => {
    vi.mocked(saveJournal).mockResolvedValue(amended("Revised support thesis"));
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole("button", { name: "Edit entry plan" }));
    await user.clear(screen.getByLabelText("Trading thesis"));
    await user.type(
      screen.getByLabelText("Trading thesis"),
      "Revised support thesis",
    );
    await user.click(
      screen.getByRole("button", { name: "Save journal revision" }),
    );
    await screen.findByText("Revised support thesis");
    expect(saveJournal).toHaveBeenCalledWith({
      executionId: 1,
      expectedVersion: 1,
      content: expect.objectContaining({
        thesis: "Revised support thesis",
        targetMicros: 10_600_000,
        stopMicros: 9_950_000,
      }),
    });
    await user.click(screen.getByText("View earlier commentary (1)"));
    expect(screen.getByText("Original support thesis")).toBeTruthy();
    expect(screen.getByText("VERSION 1 · ORIGINAL AT FILL")).toBeTruthy();
    expect(screen.getByText("24 BEN @ $10.115")).toBeTruthy();
  });
  it("records an exit reason, adherence and learning without changing fill details", async () => {
    const after = fixture();
    after.journal[0].fills[1].revisions.push({
      version: 2,
      content: {
        ...blank(),
        exitReason: "Target reached",
        followedPlan: false,
        wentWell: "Stayed patient",
        wentPoorly: "Moved the plan",
        wouldChange: "Keep the stop",
      },
      createdAt: "2026-10-03T00:00:00Z",
    });
    vi.mocked(saveJournal).mockResolvedValue(after);
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole("button", { name: "Edit exit review" }));
    await user.type(screen.getByLabelText("Exit reason"), "Target reached");
    await user.selectOptions(
      screen.getByLabelText("Did you follow your original plan?"),
      "no",
    );
    await user.type(screen.getByLabelText("What went well?"), "Stayed patient");
    await user.type(
      screen.getByLabelText("What went poorly?"),
      "Moved the plan",
    );
    await user.type(
      screen.getByLabelText("What would you change?"),
      "Keep the stop",
    );
    await user.click(
      screen.getByRole("button", { name: "Save journal revision" }),
    );
    await screen.findByText("Target reached");
    expect(saveJournal).toHaveBeenCalledWith({
      executionId: 2,
      expectedVersion: 1,
      content: expect.objectContaining({
        exitReason: "Target reached",
        followedPlan: false,
        wouldChange: "Keep the stop",
      }),
    });
    expect(screen.getByText("Net $5.64")).toBeTruthy();
  });
  it("rejects a zero target before saving and preserves unsaved commentary", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole("button", { name: "Edit entry plan" }));
    await user.clear(screen.getByLabelText("Target price (AUD)"));
    await user.type(screen.getByLabelText("Target price (AUD)"), "0");
    await user.click(
      screen.getByRole("button", { name: "Save journal revision" }),
    );
    expect((await screen.findByRole("alert")).textContent).toMatch(
      /greater than zero/,
    );
    expect(saveJournal).not.toHaveBeenCalled();
    expect(
      (screen.getByLabelText("Trading thesis") as HTMLTextAreaElement).value,
    ).toBe("Original support thesis");
  });
  it("handles a concurrent edit by offering reload instead of overwriting it", async () => {
    vi.mocked(saveJournal).mockRejectedValue(
      "This journal entry changed in another window. Reload it before saving.",
    );
    vi.mocked(getSnapshot).mockResolvedValue(
      amended("Changed in other window"),
    );
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole("button", { name: "Edit entry plan" }));
    await user.type(screen.getByLabelText("Notes"), "My draft");
    await user.click(
      screen.getByRole("button", { name: "Save journal revision" }),
    );
    await screen.findByRole("alert");
    expect((screen.getByLabelText("Notes") as HTMLTextAreaElement).value).toBe(
      "My draft",
    );
    await user.click(
      screen.getByRole("button", { name: "Reload latest journal" }),
    );
    await screen.findByText("Changed in other window");
    expect(saveJournal).toHaveBeenCalledTimes(1);
  });
  it("shows an empty state without installing example trades", () => {
    const s = fixture();
    s.journal = [];
    render(<Harness initial={s} />);
    expect(
      screen.getByRole("heading", {
        name: "Your journal starts with your first fill",
      }),
    ).toBeTruthy();
    expect(screen.queryByText("Original support thesis")).toBeNull();
  });
});
