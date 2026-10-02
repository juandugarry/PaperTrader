import { invoke, isTauri } from "@tauri-apps/api/core";
export interface Snapshot {
  displayName: string;
  profileId: string;
  notes: Note[];
  market: MarketSnapshot;
  crypto?: CryptoSnapshot;
  deposits?: DepositRecord[];
  totalContributionsMicros?: number;
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
  openMicros?: number | null;
  highMicros?: number | null;
  lowMicros?: number | null;
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
  storage?: string;
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

export interface AsxListing {
  ticker: string;
  name: string;
  kind: string;
  currency: string;
}
export interface AsxDirectory {
  entries: AsxListing[];
  fetchedAt: string | null;
}
export const getAsxDirectory = () =>
  invoke<AsxDirectory>("get_asx_directory", { profileId: activeProfile() });
export const refreshAsxDirectory = () =>
  invoke<AsxDirectory>("refresh_asx_directory", { profileId: activeProfile() });

export interface DepositRecord {
  id: number;
  account: "stocks" | "crypto";
  amountMicros: number;
  description: string;
  createdAt: string;
}
export interface CryptoWallet {
  mode: "separate" | "shared";
  cashMicros: number;
  contributionsMicros: number;
  portfolioValueMicros: number | null;
  totalReturnMicros: number | null;
  realisedPnlMicros: number;
  feeMicros: number;
}
export interface CryptoPosition {
  pair: string;
  symbol: string;
  quantityAtoms: string;
  costBasisMicros: number;
  pricePicos: string | null;
  fetchedAt: string | null;
  valueMicros: number | null;
  unrealisedPnlMicros: number | null;
}
export interface CryptoExecution {
  id: number;
  pair: string;
  symbol: string;
  side: Side;
  quantityAtoms: string;
  pricePicos: string;
  feeMicros: number;
  notionalMicros: number;
  cashDeltaMicros: number;
  notes: string;
  createdAt: string;
}
export interface CryptoSnapshot {
  wallet: CryptoWallet | null;
  positions: CryptoPosition[];
  executions: CryptoExecution[];
  assetValueMicros: number | null;
}
export interface CryptoTradeInput {
  requestId: string;
  pair: string;
  side: Side;
  quantityAtoms: string;
  pricePicos: string;
  feeMicros: number;
  notes: string;
}
export const depositVirtualFunds = (input: {
  requestId: string;
  account: "stocks" | "crypto";
  amountMicros: number;
  description: string;
}) => profileInvoke("deposit_virtual_funds", input);
export const setupCryptoWallet = (input: {
  requestId: string;
  mode: "separate" | "shared";
  startingFundsMicros: number;
}) => profileInvoke("setup_crypto_wallet", input);
export const executeCryptoTrade = (input: CryptoTradeInput) =>
  profileInvoke("execute_crypto_trade", input);

export interface CryptoAsset {
  pair: string;
  symbol: string;
  name: string;
  wsSymbol: string;
  quote: string;
}
export interface CryptoQuote {
  pair: string;
  pricePicos: string;
  usdPricePicos: string;
  fetchedAt: string;
  source: "rest" | "live";
}
export interface CryptoCandle {
  session: string;
  openPicos: string;
  highPicos: string;
  lowPicos: string;
  closePicos: string;
}
export interface CryptoChart {
  pair: string;
  candles: CryptoCandle[];
  fetchedAt: string;
}
export interface CryptoMarket {
  assets: CryptoAsset[];
  quotes: CryptoQuote[];
  charts: CryptoChart[];
  catalogFetchedAt: string | null;
  fxFetchedAt: string | null;
  lastError: string | null;
  refreshing: boolean;
}
export interface CryptoUpdate {
  market: CryptoMarket;
  snapshot: Snapshot;
}
function rememberCrypto(update: CryptoUpdate) {
  remember(update.snapshot);
  return update;
}
export const getCryptoMarket = () =>
  invoke<CryptoMarket>("get_crypto_market", { profileId: activeProfile() });
export const refreshCryptoMarket = (
  action: "catalog" | "quotes" | "history",
  pair: string | null = null,
) =>
  invoke<CryptoUpdate>("refresh_crypto_market", {
    profileId: activeProfile(),
    action,
    pair,
  }).then(rememberCrypto);
export const applyCryptoStream = (messages: string[]) =>
  invoke<CryptoUpdate>("apply_crypto_stream", {
    profileId: activeProfile(),
    messages,
  }).then(rememberCrypto);
