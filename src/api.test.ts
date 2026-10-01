import { beforeEach, describe, it, expect, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  isTauri: () => true,
}));
beforeEach(() => {
  vi.resetAllMocks();
  vi.resetModules();
});
describe("profile-scoped IPC", () => {
  it("passes the loaded profile identifier to all mutations and clears it after reset", async () => {
    const api = await import("./api");
    vi.mocked(invoke).mockResolvedValue({ profileId: "first-profile" });
    await api.getSnapshot();
    await api.saveNote({
      id: null,
      expectedVersion: null,
      title: "Research",
      body: "Question",
    });
    expect(invoke).toHaveBeenLastCalledWith("save_note", {
      profileId: "first-profile",
      input: {
        id: null,
        expectedVersion: null,
        title: "Research",
        body: "Question",
      },
    });
    await api.resetProfile("RESET");
    expect(invoke).toHaveBeenLastCalledWith("reset_profile", {
      profileId: "first-profile",
      confirmation: "RESET",
    });
    await expect(
      api.createSecurity({ ticker: "BEN", name: "Bank" }),
    ).rejects.toThrow(/Reopen/);
  });
  it("retains the existing profile context if a reset fails", async () => {
    const api = await import("./api");
    vi.mocked(invoke).mockResolvedValueOnce({ profileId: "current-profile" });
    await api.getSnapshot();
    vi.mocked(invoke).mockRejectedValueOnce("Database busy");
    await expect(api.resetProfile("RESET")).rejects.toBe("Database busy");
    vi.mocked(invoke).mockResolvedValueOnce({ profileId: "current-profile" });
    await api.setPrice({ securityId: 1, priceMicros: 10_000_000 });
    expect(invoke).toHaveBeenLastCalledWith("set_price", {
      profileId: "current-profile",
      input: { securityId: 1, priceMicros: 10_000_000 },
    });
  });
  it("rejects a profile generation change on reload instead of applying an old draft to new IDs", async () => {
    const api = await import("./api");
    vi.mocked(invoke).mockResolvedValueOnce({ profileId: "old" });
    await api.getSnapshot();
    vi.mocked(invoke).mockResolvedValueOnce({ profileId: "new" });
    await expect(api.getSnapshot()).rejects.toThrow(/reset or changed/);
    vi.mocked(invoke).mockResolvedValueOnce({ profileId: "old" });
    await api.setPrice({ securityId: 1, priceMicros: 10_000_000 });
    expect(invoke).toHaveBeenLastCalledWith("set_price", {
      profileId: "old",
      input: { securityId: 1, priceMicros: 10_000_000 },
    });
  });
  it("passes profile identity to credential and refresh commands without requesting secret values", async () => {
    const api = await import("./api");
    vi.mocked(invoke).mockResolvedValueOnce({ profileId: "profile" });
    await api.getSnapshot();
    vi.mocked(invoke).mockResolvedValueOnce({
      configured: true,
      supported: true,
    });
    await api.marketKeyStatus();
    expect(invoke).toHaveBeenLastCalledWith("market_key_status", {
      profileId: "profile",
    });
    vi.mocked(invoke).mockResolvedValueOnce({ profileId: "profile" });
    await api.refreshMarketPrices([1, 2]);
    expect(invoke).toHaveBeenLastCalledWith("refresh_market_prices", {
      profileId: "profile",
      securityIds: [1, 2],
    });
    vi.mocked(invoke).mockResolvedValueOnce(undefined);
    await api.removeMarketKey();
    expect(invoke).toHaveBeenLastCalledWith("remove_market_key", {
      profileId: "profile",
    });
  });
});
