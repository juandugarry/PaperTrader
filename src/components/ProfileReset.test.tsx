// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import ProfileReset from "./ProfileReset";
import { resetProfile } from "../api";
vi.mock("../api", () => ({ resetProfile: vi.fn() }));
beforeEach(() => vi.resetAllMocks());
afterEach(cleanup);
describe("profile reset confirmation", () => {
  it("requires opening confirmation and typing RESET before any deletion", async () => {
    const user = userEvent.setup(),
      onReset = vi.fn();
    vi.mocked(resetProfile).mockResolvedValue();
    render(<ProfileReset onReset={onReset} />);
    await user.click(screen.getByRole("button", { name: "Reset profile…" }));
    expect(resetProfile).not.toHaveBeenCalled();
    expect(
      (
        screen.getByRole("button", {
          name: "Permanently reset profile",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
    await user.type(screen.getByLabelText("Type RESET to confirm"), "RESET");
    await user.click(
      screen.getByRole("button", { name: "Permanently reset profile" }),
    );
    expect(resetProfile).toHaveBeenCalledWith("RESET");
    expect(onReset).toHaveBeenCalledOnce();
  });
  it("cancels without resetting and surfaces reset failures without leaving the profile", async () => {
    const user = userEvent.setup(),
      onReset = vi.fn();
    vi.mocked(resetProfile).mockRejectedValue("Database busy");
    render(<ProfileReset onReset={onReset} />);
    await user.click(screen.getByRole("button", { name: "Reset profile…" }));
    await user.click(screen.getByRole("button", { name: "Keep my profile" }));
    expect(resetProfile).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Reset profile…" }));
    await user.type(screen.getByLabelText("Type RESET to confirm"), "RESET");
    await user.click(
      screen.getByRole("button", { name: "Permanently reset profile" }),
    );
    expect((await screen.findByRole("alert")).textContent).toBe(
      "Database busy",
    );
    expect(onReset).not.toHaveBeenCalled();
  });
});
