// @vitest-environment jsdom
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import Notepad from "./Notepad";
import { saveNote, deleteNote, getSnapshot, type Snapshot } from "../api";
vi.mock("../api", () => ({
  saveNote: vi.fn(),
  deleteNote: vi.fn(),
  getSnapshot: vi.fn(),
}));
function fixture(): Snapshot {
  return {
    displayName: "Alex",
    profileId: "test-profile",
    notes: [],
    market: {
      requestsToday: 0,
      dailyLimit: 20,
      refreshing: false,
      histories: [],
    },
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
function noted() {
  const s = fixture();
  s.notes = [
    {
      id: 1,
      title: "Support observations",
      body: "Review volume near support",
      version: 1,
      createdAt: s.createdAt,
      updatedAt: s.createdAt,
    },
  ];
  return s;
}
function Harness({ initial = fixture() }: { initial?: Snapshot }) {
  const [snapshot, setSnapshot] = useState(initial);
  return <Notepad snapshot={snapshot} onChanged={setSnapshot} />;
}
beforeEach(() => vi.resetAllMocks());
afterEach(cleanup);
describe("local notepad", () => {
  it("creates a local note and shows saved content", async () => {
    vi.mocked(saveNote).mockResolvedValue(noted());
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole("button", { name: "New note" }));
    await user.type(
      screen.getByLabelText("Note title"),
      "Support observations",
    );
    await user.type(
      screen.getByLabelText("Note text"),
      "Review volume near support",
    );
    await user.click(screen.getByRole("button", { name: "Save note" }));
    await screen.findByRole("button", { name: /Support observations/ });
    expect(saveNote).toHaveBeenCalledWith({
      id: null,
      expectedVersion: null,
      title: "Support observations",
      body: "Review volume near support",
    });
    expect(
      (screen.getByLabelText("Note text") as HTMLTextAreaElement).value,
    ).toBe("Review volume near support");
  });
  it("edits with the displayed version and searches titles/body", async () => {
    const after = noted();
    after.notes[0].version = 2;
    after.notes[0].body = "Review brokerage too";
    vi.mocked(saveNote).mockResolvedValue(after);
    const user = userEvent.setup();
    render(<Harness initial={noted()} />);
    await user.clear(screen.getByLabelText("Note text"));
    await user.type(screen.getByLabelText("Note text"), "Review brokerage too");
    await user.click(screen.getByRole("button", { name: "Save note" }));
    await screen.findByRole("button", { name: /Review brokerage too/ });
    expect(saveNote).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 1,
        expectedVersion: 1,
        body: "Review brokerage too",
      }),
    );
    await user.type(screen.getByLabelText("Search notes"), "nothing-matches");
    expect(screen.getByText("No matching notes.")).toBeTruthy();
  });
  it("requires an explicit delete confirmation and permits cancellation", async () => {
    vi.mocked(deleteNote).mockResolvedValue(fixture());
    const user = userEvent.setup();
    render(<Harness initial={noted()} />);
    await user.click(screen.getByRole("button", { name: "Delete note…" }));
    expect(deleteNote).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Keep note" }));
    expect(deleteNote).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Delete note…" }));
    await user.click(screen.getByRole("button", { name: "Delete this note" }));
    await screen.findByRole("heading", {
      name: "Make space for your thinking",
    });
    expect(deleteNote).toHaveBeenCalledWith({ id: 1, expectedVersion: 1 });
  });
  it("retains a draft on save failure and offers a conflict reload", async () => {
    vi.mocked(saveNote).mockRejectedValue(
      "This note changed in another window. Reload it before saving.",
    );
    vi.mocked(getSnapshot).mockResolvedValue(noted());
    const user = userEvent.setup();
    render(<Harness initial={noted()} />);
    await user.type(screen.getByLabelText("Note text"), " Draft");
    await user.click(screen.getByRole("button", { name: "Save note" }));
    await screen.findByRole("alert");
    expect(
      (screen.getByLabelText("Note text") as HTMLTextAreaElement).value,
    ).toContain("Draft");
    expect(
      screen.getByRole("button", { name: "Reload latest notes" }),
    ).toBeTruthy();
  });
});
