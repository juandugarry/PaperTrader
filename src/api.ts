import { invoke, isTauri } from "@tauri-apps/api/core";
export interface Snapshot {
  displayName: string;
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
export const getSnapshot = () => invoke<Snapshot | null>("get_snapshot");
export const createProfile = (input: CreateProfile) =>
  invoke<Snapshot>("create_profile", { input });
export type Side = "BUY" | "SELL";
export interface Security {
  id: number;
  ticker: string;
  name: string;
  currentPriceMicros: number | null;
  priceUpdatedAt: string | null;
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
  invoke<Snapshot>("create_security", { input });
export const setPrice = (input: { securityId: number; priceMicros: number }) =>
  invoke<Snapshot>("set_price", { input });
export const executeTrade = (input: TradeInput) =>
  invoke<Snapshot>("execute_trade", { input });
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
}) => invoke<Snapshot>("save_journal", { input });
