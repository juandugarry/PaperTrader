import { invoke, isTauri } from "@tauri-apps/api/core";
export interface Snapshot {
  displayName: string;
  profileId: string;
  notes: Note[];
  market: MarketSnapshot;
  defaultBrokerageMicros: number;
  currency: string;
  primaryMarket: string;
  startingCapitalMicros: number;
  cashMicros: number;
  createdAt: string;
  trading: TradingSnapshot;
  journal: JournalTrade[];
}
export interface CreateProfile {
  displayName: string;
  startingCapitalMicros: number;
  defaultBrokerageMicros: number;
}
export const desktopAvailable = isTauri();
let currentProfileId: string | null = null;
function remember<T extends Snapshot | null>(snapshot: T): T {
  if (currentProfileId && snapshot && snapshot.profileId !== currentProfileId) {
    throw new Error(
      "The profile was reset or changed. Reopen the app before continuing.",
    );
  }
  currentProfileId = snapshot?.profileId ?? null;
  return snapshot;
}
function profileInvoke(command: string, input: unknown): Promise<Snapshot> {
  if (!currentProfileId)
    return Promise.reject(
      new Error("Reopen your local profile before continuing."),
    );
  return invoke<Snapshot>(command, { profileId: currentProfileId, input }).then(
    remember,
  );
}
export const getSnapshot = () =>
  invoke<Snapshot | null>("get_snapshot").then(remember);
export const createProfile = (input: CreateProfile) =>
  invoke<Snapshot>("create_profile", { input }).then(remember);
export type Side = "BUY" | "SELL";
export interface Security {
  id: number;
  ticker: string;
  name: string;
  currentPriceMicros: number | null;
  priceUpdatedAt: string | null;
  priceSource: "manual" | "eodhd";
  priceAsOf: string | null;
  marketFetchedAt: string | null;
  marketError: string | null;
}
export interface Position {
  securityId: number;
  ticker: string;
  name: string;
  quantity: number;
  averageEntryMicros: number;
  costBasisMicros: number;
  currentPriceMicros: number | null;
  priceUpdatedAt: string | null;
  valueMicros: number | null;
  unrealisedPnlMicros: number | null;
}
export interface Execution {
  id: number;
  securityId: number;
  ticker: string;
  name: string;
  side: Side;
  quantity: number;
  priceMicros: number;
  brokerageMicros: number;
  notionalMicros: number;
  cashDeltaMicros: number;
  createdAt: string;
}
export interface TradingSnapshot {
  securities: Security[];
  positions: Position[];
  executions: Execution[];
  capitalInvestedMicros: number;
  realisedPnlMicros: number;
  brokeragePaidMicros: number;
  unrealisedPnlMicros: number | null;
  portfolioValueMicros: number | null;
  totalReturnMicros: number | null;
}
export interface TradeInput {
  journal?: JournalContent;
  requestId: string;
  securityId: number;
  side: Side;
  quantity: number;
  priceMicros: number;
  brokerageMicros: number;
}
export const createSecurity = (input: { ticker: string; name: string }) =>
  profileInvoke("create_security", input);
export const setPrice = (input: { securityId: number; priceMicros: number }) =>
  profileInvoke("set_price", input);
export const executeTrade = (input: TradeInput) =>
  profileInvoke("execute_trade", input);
export interface JournalContent {
  thesis: string;
  entryTrigger: string;
  targetMicros: number | null;
  stopMicros: number | null;
  plannedRiskMicros: number | null;
  notes: string;
  exitReason: string;
  followedPlan: boolean | null;
  wentWell: string;
  wentPoorly: string;
  wouldChange: string;
}
export interface JournalRevision {
  version: number;
  content: JournalContent;
  createdAt: string;
}
export interface JournalFill {
  execution: Execution;
  capturedWithFill: boolean;
  revisions: JournalRevision[];
  grossPnlMicros: number | null;
  netPnlMicros: number | null;
  transactionCostsMicros: number | null;
  releasedCostMicros: number | null;
}
export interface JournalTrade {
  id: number;
  securityId: number;
  ticker: string;
  name: string;
  openedAt: string;
  closedAt: string | null;
  quantityBought: number;
  quantitySold: number;
  quantityOpen: number;
  entryCostMicros: number;
  brokeragePaidMicros: number;
  grossPnlMicros: number;
  netPnlMicros: number;
  realisedCostsMicros: number;
  realisedBasisMicros: number;
  fills: JournalFill[];
}
export const saveJournal = (input: {
  executionId: number;
  expectedVersion: number;
  content: JournalContent;
}) => profileInvoke("save_journal", input);

export interface Note {
  id: number;
  title: string;
  body: string;
  version: number;
  createdAt: string;
  updatedAt: string;
}
export const saveNote = (input: {
  id: number | null;
  expectedVersion: number | null;
  title: string;
  body: string;
}) => profileInvoke("save_note", input);
export const deleteNote = (input: { id: number; expectedVersion: number }) =>
  profileInvoke("delete_note", input);
export async function resetProfile(confirmation: string): Promise<void> {
  if (!currentProfileId)
    throw new Error("Reopen your local profile before resetting it.");
  await invoke<void>("reset_profile", {
    profileId: currentProfileId,
    confirmation,
  });
  currentProfileId = null;
}

export interface DailyPrice {
  sessionDate: string;
  closeMicros: number;
  adjustedCloseMicros: number | null;
}
export interface PriceHistory {
  securityId: number;
  fetchedAt: string;
  prices: DailyPrice[];
}
export interface MarketSnapshot {
  requestsToday: number;
  dailyLimit: number;
  refreshing: boolean;
  histories: PriceHistory[];
}
export interface MarketKeyStatus {
  configured: boolean;
  supported: boolean;
}
function activeProfile(): string {
  if (!currentProfileId)
    throw new Error("Reopen your local profile before continuing.");
  return currentProfileId;
}
export const marketKeyStatus = () =>
  invoke<MarketKeyStatus>("market_key_status", { profileId: activeProfile() });
export const saveMarketKey = (key: string) =>
  invoke<void>("save_market_key", { profileId: activeProfile(), key });
export const removeMarketKey = () =>
  invoke<void>("remove_market_key", { profileId: activeProfile() });
export const refreshMarketPrices = (securityIds: number[]) =>
  invoke<Snapshot>("refresh_market_prices", {
    profileId: activeProfile(),
    securityIds,
  }).then(remember);

export const openMarketSignup = () => invoke<void>("open_market_signup");
