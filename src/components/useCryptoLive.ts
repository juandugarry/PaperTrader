import { useEffect, useRef, useState } from "react";
import { applyCryptoStream, type CryptoUpdate } from "../api";
export default function useCryptoLive(
  enabled: boolean,
  symbols: string[],
  onUpdate: (update: CryptoUpdate) => void,
) {
  const [status, setStatus] = useState("Live updates off"),
    socketRef = useRef<WebSocket | null>(null),
    subscribed = useRef<string[]>([]),
    symbolsRef = useRef(symbols),
    onUpdateRef = useRef(onUpdate);
  symbolsRef.current = symbols;
  onUpdateRef.current = onUpdate;
  const signature = [...new Set(symbols)].sort().join(",");
  useEffect(() => {
    const socket = socketRef.current;
    if (!socket || socket.readyState !== WebSocket.OPEN) return;
    const old = subscribed.current,
      next = [...new Set(symbolsRef.current)];
    const remove = old.filter((s) => !next.includes(s)),
      add = next.filter((s) => !old.includes(s));
    if (remove.length)
      socket.send(
        JSON.stringify({
          method: "unsubscribe",
          params: { channel: "ticker", symbol: remove },
        }),
      );
    if (add.length)
      socket.send(
        JSON.stringify({
          method: "subscribe",
          params: { channel: "ticker", symbol: add, snapshot: true },
        }),
      );
    subscribed.current = next;
  }, [signature]);
  useEffect(() => {
    if (!enabled) {
      setStatus("Live updates off");
      return;
    }
    let active = true,
      retry: ReturnType<typeof setTimeout> | null = null,
      attempts = 0,
      inFlight = false;
    const pending = new Map<string, string>();
    function connect() {
      if (!active || document.hidden) {
        setStatus("Live updates paused while the app is in the background");
        return;
      }
      setStatus("Connecting to Kraken live feed…");
      const socket = new WebSocket("wss://ws.kraken.com/v2");
      socketRef.current = socket;
      subscribed.current = [];
      socket.onopen = () => {
        if (!active) {
          socket.close();
          return;
        }
        setStatus("Live feed connected · selected coin and wallet holdings");
        const next = [...new Set(symbolsRef.current)];
        if (next.length)
          socket.send(
            JSON.stringify({
              method: "subscribe",
              params: { channel: "ticker", symbol: next, snapshot: true },
            }),
          );
        subscribed.current = next;
      };
      socket.onmessage = (event) => {
        if (!active || typeof event.data !== "string") return;
        try {
          const value = JSON.parse(event.data);
          if (value.success === false) {
            setStatus(
              "Kraken could not subscribe to a market. Cached prices remain available.",
            );
            return;
          }
          if (value.channel === "ticker" && Array.isArray(value.data)) {
            const key = value.data
              .map((v: { symbol: string }) => v.symbol)
              .join(",");
            pending.set(key, event.data);
            if (pending.size > 100)
              pending.delete(pending.keys().next().value!);
          }
        } catch {
          setStatus("Could not read live update. Cached prices kept.");
        }
      };
      socket.onerror = () =>
        setStatus("Live feed unavailable · cached prices kept");
      socket.onclose = () => {
        if (!active || document.hidden || socket !== socketRef.current) return;
        setStatus("Live feed disconnected · reconnecting with backoff");
        retry = setTimeout(
          connect,
          Math.min(60000, 5000 * 2 ** Math.min(attempts++, 4)),
        );
      };
    }
    const flush = setInterval(() => {
      if (!active || document.hidden || inFlight || !pending.size) return;
      const messages = [...pending.values()];
      pending.clear();
      inFlight = true;
      void applyCryptoStream(messages)
        .then((update) => {
          if (active) onUpdateRef.current(update);
        })
        .catch((e) => {
          if (active) setStatus(`Live price update unavailable: ${String(e)}`);
        })
        .finally(() => {
          inFlight = false;
        });
    }, 1000);
    const visibility = () => {
      if (retry) {
        clearTimeout(retry);
        retry = null;
      }
      if (document.hidden) {
        socketRef.current?.close();
        pending.clear();
        setStatus("Live updates paused while the app is in the background");
      } else if (
        !socketRef.current ||
        socketRef.current.readyState === WebSocket.CLOSED
      ) {
        connect();
      }
    };
    document.addEventListener("visibilitychange", visibility);
    connect();
    return () => {
      active = false;
      clearInterval(flush);
      if (retry) clearTimeout(retry);
      document.removeEventListener("visibilitychange", visibility);
      socketRef.current?.close();
      socketRef.current = null;
      pending.clear();
    };
  }, [enabled]);
  return status;
}
