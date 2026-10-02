// @vitest-environment jsdom
import { afterEach, it, expect, vi } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import VirtualDeposit from "./VirtualDeposit";
import { depositVirtualFunds, type Snapshot } from "../api";
vi.mock("../api", () => ({ depositVirtualFunds: vi.fn() }));
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
it("reviews a fictional deposit before submitting exact cents", async () => {
  const onChanged = vi.fn();
  vi.mocked(depositVirtualFunds).mockResolvedValue({
    cashMicros: 1500000000,
  } as Snapshot);
  render(<VirtualDeposit account="stocks" onChanged={onChanged} />);
  const u = userEvent.setup();
  await u.click(screen.getByRole("button", { name: "Deposit virtual funds" }));
  await u.type(screen.getByLabelText("Virtual deposit amount (AUD)"), "500.25");
  await u.click(screen.getByRole("button", { name: "Review virtual deposit" }));
  expect(depositVirtualFunds).not.toHaveBeenCalled();
  await u.click(
    screen.getByRole("button", { name: "Confirm virtual deposit" }),
  );
  expect(depositVirtualFunds).toHaveBeenCalledWith(
    expect.objectContaining({ account: "stocks", amountMicros: 500250000 }),
  );
  expect(onChanged).toHaveBeenCalledOnce();
});
