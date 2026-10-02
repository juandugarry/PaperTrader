// @vitest-environment jsdom
import { afterEach, beforeEach, it, expect, vi } from "vitest";
import { renderHook, act, cleanup } from "@testing-library/react";
import useCryptoLive from "./useCryptoLive";
import { applyCryptoStream, type CryptoUpdate } from "../api";
vi.mock("../api", () => ({ applyCryptoStream: vi.fn() }));
const sockets: Socket[] = [];
class Socket {
  static OPEN = 1;
  static CLOSED = 3;
  readyState = 0;
  send = vi.fn();
  onopen: (() => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  constructor(public url: string) {
    sockets.push(this);
  }
  open() {
    this.readyState = 1;
    this.onopen?.();
  }
  message(data: string) {
    this.onmessage?.({ data });
  }
  close() {
    this.readyState = 3;
    this.onclose?.();
  }
}
let hidden = false;
beforeEach(() => {
  vi.useFakeTimers();
  sockets.length = 0;
  hidden = false;
  Object.defineProperty(document, "hidden", {
    configurable: true,
    get: () => hidden,
  });
  vi.stubGlobal("WebSocket", Socket);
  vi.mocked(applyCryptoStream).mockResolvedValue({
    market: {},
    snapshot: {},
  } as CryptoUpdate);
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});
it("keeps one connection, preserves raw decimal messages and changes subscriptions without reconnecting", async () => {
  const update = vi.fn();
  const { rerender } = renderHook(
    ({ symbols }) => useCryptoLive(true, symbols, update),
    { initialProps: { symbols: ["AUD/USD", "BTC/USD"] } },
  );
  expect(sockets).toHaveLength(1);
  act(() => sockets[0].open());
  const raw =
    '{"channel":"ticker","type":"update","data":[{"symbol":"BTC/USD","last":100000.000000000001,"timestamp":"2026-10-02T00:00:00Z"}]}';
  act(() => {
    sockets[0].message(raw);
    sockets[0].message(raw);
  });
  await act(async () => {
    vi.advanceTimersByTime(1000);
    await Promise.resolve();
  });
  expect(applyCryptoStream).toHaveBeenCalledTimes(1);
  expect(applyCryptoStream).toHaveBeenCalledWith([raw]);
  expect(update).toHaveBeenCalledOnce();
  rerender({ symbols: ["AUD/USD", "ETH/USD"] });
  expect(sockets).toHaveLength(1);
  expect(sockets[0].send).toHaveBeenCalledWith(
    JSON.stringify({
      method: "unsubscribe",
      params: { channel: "ticker", symbol: ["BTC/USD"] },
    }),
  );
});
it("pauses in the background and reconnects with backoff after a disconnect", () => {
  const { unmount } = renderHook(() =>
    useCryptoLive(true, ["BTC/USD", "AUD/USD"], vi.fn()),
  );
  act(() => sockets[0].open());
  act(() => sockets[0].close());
  act(() => vi.advanceTimersByTime(4999));
  expect(sockets).toHaveLength(1);
  act(() => vi.advanceTimersByTime(1));
  expect(sockets).toHaveLength(2);
  act(() => sockets[1].open());
  act(() => {
    hidden = true;
    document.dispatchEvent(new Event("visibilitychange"));
    vi.advanceTimersByTime(60000);
  });
  expect(sockets).toHaveLength(2);
  expect(applyCryptoStream).not.toHaveBeenCalled();
  unmount();
});
