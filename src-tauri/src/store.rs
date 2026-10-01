use crate::domain::validate_setup;
use crate::journal::{self, JournalTrade, SaveJournal};
use crate::market::{self, DailyPrice, History, MarketSnapshot, RefreshJob, RefreshSecurity};
use crate::notes::{self, DeleteNote, Note, SaveNote};
use crate::trading::{self, CreateSecurity, ExecuteTrade, SetPrice, TradingSnapshot};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProfile {
    pub display_name: String,
    pub starting_capital_micros: i64,
    pub default_brokerage_micros: i64,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub display_name: String,
    pub profile_id: String,
    pub notes: Vec<Note>,
    pub market: MarketSnapshot,
    pub default_brokerage_micros: i64,
    pub currency: String,
    pub primary_market: String,
    pub starting_capital_micros: i64,
    pub cash_micros: i64,
    pub created_at: String,
    pub trading: TradingSnapshot,
    pub journal: Vec<JournalTrade>,
}
pub enum ProfileAction {
    CreateSecurity(CreateSecurity),
    SetPrice(SetPrice),
    ExecuteTrade(ExecuteTrade),
    SaveJournal(SaveJournal),
    SaveNote(SaveNote),
    DeleteNote(DeleteNote),
}
pub struct Store {
    connection: Connection,
    credential_path: Option<PathBuf>,
}
impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let mut store = Self::from_connection(Connection::open(path).map_err(|e| e.to_string())?)?;
        store.credential_path = Some(path.with_file_name("eodhd-api-key.txt"));
        Ok(store)
    }
    fn from_connection(mut connection: Connection) -> Result<Self, String> {
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| e.to_string())?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
            .map_err(|e| e.to_string())?;
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if version > 7 {
            return Err("This database is newer than this version of PaperTrader.".into());
        }
        if version == 0 {
            let tx = connection.transaction().map_err(|e| e.to_string())?;
            tx.execute_batch(include_str!("../migrations/001_foundation.sql"))
                .map_err(|e| e.to_string())?;
            tx.pragma_update(None, "user_version", 1)
                .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
        }
        if version < 2 {
            let tx = connection.transaction().map_err(|e| e.to_string())?;
            tx.execute_batch(include_str!("../migrations/002_manual_trading.sql"))
                .map_err(|e| e.to_string())?;
            tx.pragma_update(None, "user_version", 2)
                .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
        }
        if version < 3 {
            let tx = connection.transaction().map_err(|e| e.to_string())?;
            tx.execute_batch(include_str!("../migrations/003_trading_journal.sql"))
                .map_err(|e| e.to_string())?;
            tx.pragma_update(None, "user_version", 3)
                .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
        }
        if version < 4 {
            let tx = connection.transaction().map_err(|e| e.to_string())?;
            tx.execute_batch(include_str!("../migrations/004_notes_and_reset.sql"))
                .map_err(|e| e.to_string())?;
            tx.pragma_update(None, "user_version", 4)
                .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
        }
        if version < 5 {
            let tx = connection.transaction().map_err(|e| e.to_string())?;
            tx.execute_batch(include_str!("../migrations/005_market_data.sql"))
                .map_err(|e| e.to_string())?;
            tx.pragma_update(None, "user_version", 5)
                .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
        }
        if version < 6 {
            let tx = connection.transaction().map_err(|e| e.to_string())?;
            tx.execute_batch(include_str!("../migrations/006_directory.sql"))
                .map_err(|e| e.to_string())?;
            tx.pragma_update(None, "user_version", 6)
                .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
        }
        if version < 7 {
            let tx = connection.transaction().map_err(|e| e.to_string())?;
            tx.execute_batch(include_str!("../migrations/007_candles.sql"))
                .map_err(|e| e.to_string())?;
            tx.pragma_update(None, "user_version", 7)
                .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
        }
        Ok(Self {
            connection,
            credential_path: None,
        })
    }
    pub fn snapshot(&self) -> Result<Option<Snapshot>, String> {
        // Keep metadata, ledger and execution reads in one consistent read transaction.
        let tx = self
            .connection
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let metadata=tx.query_row(
            "SELECT s.display_name,s.default_brokerage_micros,s.currency,s.primary_market,
            (SELECT COALESCE(SUM(amount_micros),0) FROM cash_ledger WHERE portfolio_id=1 AND kind='opening_capital'),
            (SELECT COALESCE(SUM(amount_micros),0) FROM cash_ledger WHERE portfolio_id=1),p.created_at,s.profile_id
            FROM settings s JOIN portfolios p ON p.id=s.id WHERE s.id=1", [],
            |r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,i64>(4)?,r.get::<_,i64>(5)?,r.get::<_,String>(6)?,r.get::<_,String>(7)?))
        ).optional().map_err(|e|e.to_string())?;
        let Some((
            display_name,
            default_brokerage_micros,
            currency,
            primary_market,
            starting_capital_micros,
            cash_micros,
            created_at,
            profile_id,
        )) = metadata
        else {
            return Ok(None);
        };
        let trading = trading::read(&tx, cash_micros, starting_capital_micros)?;
        let journal = journal::read(&tx)?;
        let notes = notes::read(&tx)?;
        let market = Self::read_market(&tx)?;
        Ok(Some(Snapshot {
            display_name,
            profile_id,
            notes,
            market,
            default_brokerage_micros,
            currency,
            primary_market,
            starting_capital_micros,
            cash_micros,
            created_at,
            trading,
            journal,
        }))
    }
    fn mutate(
        &mut self,
        action: impl FnOnce(&Connection) -> Result<(), String>,
    ) -> Result<Snapshot, String> {
        self.mutate_profile(None, action)
    }
    fn mutate_profile(
        &mut self,
        profile_id: Option<&str>,
        action: impl FnOnce(&Connection) -> Result<(), String>,
    ) -> Result<Snapshot, String> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if !tx
            .query_row("SELECT EXISTS(SELECT 1 FROM settings)", [], |r| {
                r.get::<_, bool>(0)
            })
            .map_err(|e| e.to_string())?
        {
            return Err("Create your local profile first.".into());
        }
        if let Some(expected) = profile_id {
            Self::check_profile(&tx, expected)?;
        }
        action(&tx)?;
        let (cash,starting):(i64,i64)=tx.query_row("SELECT SUM(amount_micros),SUM(CASE WHEN kind='opening_capital' THEN amount_micros ELSE 0 END) FROM cash_ledger WHERE portfolio_id=1",[],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
        // Check all derived totals and limits before committing the mutation.
        trading::read(&tx, cash, starting)?;
        journal::read(&tx)?;
        tx.commit().map_err(|e| e.to_string())?;
        self.snapshot()?
            .ok_or_else(|| "Portfolio unavailable".into())
    }
    pub fn create_security(&mut self, input: CreateSecurity) -> Result<Snapshot, String> {
        self.mutate(|c| trading::create_security(c, input))
    }
    pub fn set_price(&mut self, input: SetPrice) -> Result<Snapshot, String> {
        self.mutate(|c| trading::set_price(c, input))
    }
    pub fn execute_trade(&mut self, input: ExecuteTrade) -> Result<Snapshot, String> {
        self.mutate(|c| trading::execute(c, input))
    }
    pub fn save_journal(&mut self, input: SaveJournal) -> Result<Snapshot, String> {
        self.mutate(|c| journal::save(c, input))
    }

    pub(crate) fn check_profile(c: &Connection, expected: &str) -> Result<(), String> {
        let actual = c
            .query_row("SELECT profile_id FROM settings WHERE id=1", [], |r| {
                r.get::<_, String>(0)
            })
            .optional()
            .map_err(|e| e.to_string())?;
        if actual.as_deref() != Some(expected) {
            return Err(
                "The profile was reset or changed. Reopen the app before continuing.".into(),
            );
        }
        Ok(())
    }
    pub fn apply_profile_action(
        &mut self,
        profile_id: &str,
        action: ProfileAction,
    ) -> Result<Snapshot, String> {
        self.mutate_profile(Some(profile_id), |c| match action {
            ProfileAction::CreateSecurity(input) => trading::create_security(c, input),
            ProfileAction::SetPrice(input) => trading::set_price(c, input),
            ProfileAction::ExecuteTrade(input) => trading::execute(c, input),
            ProfileAction::SaveJournal(input) => journal::save(c, input),
            ProfileAction::SaveNote(input) => notes::save(c, input),
            ProfileAction::DeleteNote(input) => notes::delete(c, input),
        })
    }
    pub fn reset_profile(&mut self, profile_id: &str, confirmation: &str) -> Result<(), String> {
        if confirmation != "RESET" {
            return Err("Type RESET to confirm permanently clearing all local progress.".into());
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        Self::check_profile(&tx, profile_id)?;
        // Explicit whole-profile reset only. Ordinary fill and journal edits retain their immutability triggers.
        tx.execute_batch("DROP TABLE asx_directory; DROP TABLE daily_prices; DROP TABLE journal_revisions; DROP TABLE journal_entries; DROP TABLE notes; DROP TABLE cash_ledger; DROP TABLE executions; DROP TABLE securities; DROP TABLE portfolios; DROP TABLE settings;").map_err(|e|e.to_string())?;
        tx.execute_batch(include_str!("../migrations/001_foundation.sql"))
            .map_err(|e| e.to_string())?;
        tx.execute_batch(include_str!("../migrations/002_manual_trading.sql"))
            .map_err(|e| e.to_string())?;
        tx.execute_batch(include_str!("../migrations/003_trading_journal.sql"))
            .map_err(|e| e.to_string())?;
        tx.execute_batch(include_str!("../migrations/004_notes_and_reset.sql"))
            .map_err(|e| e.to_string())?;
        tx.execute_batch(include_str!("../migrations/005_market_data.sql"))
            .map_err(|e| e.to_string())?;
        tx.execute_batch(include_str!("../migrations/006_directory.sql"))
            .map_err(|e| e.to_string())?;
        tx.execute_batch(include_str!("../migrations/007_candles.sql"))
            .map_err(|e| e.to_string())?;
        tx.pragma_update(None, "user_version", 7)
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }
    fn read_market(c: &Connection) -> Result<MarketSnapshot, String> {
        let requests_today = c
            .query_row(
                "SELECT COALESCE((SELECT requests FROM market_usage WHERE utc_day=date('now')),0)",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let refreshing = c.query_row("SELECT market_refresh_id IS NOT NULL AND market_refresh_started > unixepoch()-600 FROM settings WHERE id=1", [], |r| r.get(0)).map_err(|e| e.to_string())?;
        let mut histories = Vec::new();
        let mut query = c.prepare("SELECT id,market_fetched_at FROM securities WHERE market_fetched_at IS NOT NULL ORDER BY ticker").map_err(|e|e.to_string())?;
        let securities = query
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        for security in securities {
            let (security_id, fetched_at) = security.map_err(|e| e.to_string())?;
            let mut q = c.prepare("SELECT session_date,close_micros,adjusted_close_micros,open_micros,high_micros,low_micros FROM daily_prices WHERE security_id=?1 ORDER BY session_date").map_err(|e|e.to_string())?;
            let prices = q
                .query_map([security_id], |r| {
                    Ok(DailyPrice {
                        session_date: r.get(0)?,
                        close_micros: r.get(1)?,
                        adjusted_close_micros: r.get(2)?,
                        open_micros: r.get(3)?,
                        high_micros: r.get(4)?,
                        low_micros: r.get(5)?,
                    })
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            histories.push(History {
                security_id,
                fetched_at,
                prices,
            });
        }
        Ok(MarketSnapshot {
            requests_today,
            daily_limit: market::DAILY_LIMIT,
            refreshing,
            histories,
        })
    }
    pub fn credential_path(&self) -> Result<&Path, String> {
        self.credential_path
            .as_deref()
            .ok_or_else(|| "Local API key storage unavailable.".into())
    }
    pub fn verify_profile(&self, profile_id: &str) -> Result<(), String> {
        Self::check_profile(&self.connection, profile_id)
    }
    fn reserve_market_requests(tx: &Connection, count: i64) -> Result<String, String> {
        let (active, started): (Option<String>, Option<i64>) = tx
            .query_row(
                "SELECT market_refresh_id,market_refresh_started FROM settings WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().timestamp();
        if active.is_some() && started.is_some_and(|t| now - t < 600) {
            return Err("A market-data refresh is already running. Wait for it to finish.".into());
        }
        if started.is_some_and(|t| now - t < 60) {
            return Err(
                "Wait one minute between market-data refreshes to protect your allowance.".into(),
            );
        }
        let used: i64 = tx
            .query_row(
                "SELECT COALESCE((SELECT requests FROM market_usage WHERE utc_day=date('now')),0)",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if used + count > market::DAILY_LIMIT {
            return Err(format!("Only {} local requests remain today. Refresh fewer securities or wait until tomorrow (UTC).",market::DAILY_LIMIT-used));
        }
        let id: String = tx
            .query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO market_usage(utc_day,requests) VALUES(date('now'),?1) ON CONFLICT(utc_day) DO UPDATE SET requests=requests+excluded.requests",[count]).map_err(|e|e.to_string())?;
        tx.execute(
            "DELETE FROM market_usage WHERE utc_day<date('now','-7 days')",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE settings SET market_refresh_id=?1,market_refresh_started=?2 WHERE id=1",
            params![id, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(id)
    }
    pub fn directory(&self, profile_id: &str) -> Result<crate::directory::Directory, String> {
        Self::check_profile(&self.connection, profile_id)?;
        let row: Option<(String, String)> = self
            .connection
            .query_row(
                "SELECT body,fetched_at FROM asx_directory WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        match row {
            Some((body, fetched_at)) => Ok(crate::directory::Directory {
                entries: serde_json::from_str(&body).map_err(|_| "Invalid cached directory")?,
                fetched_at: Some(fetched_at),
            }),
            None => Ok(crate::directory::Directory {
                entries: vec![],
                fetched_at: None,
            }),
        }
    }
    pub fn begin_directory(&mut self, profile_id: &str) -> Result<String, String> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        Self::check_profile(&tx, profile_id)?;
        let id = Self::reserve_market_requests(&tx, 1)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(id)
    }
    pub fn finish_directory(
        &mut self,
        profile_id: &str,
        job: &str,
        result: Result<Vec<crate::directory::Listing>, String>,
    ) -> Result<crate::directory::Directory, String> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        Self::check_profile(&tx, profile_id)?;
        let active: Option<String> = tx
            .query_row(
                "SELECT market_refresh_id FROM settings WHERE id=1",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if active.as_deref() != Some(job) {
            return Err("This directory refresh is no longer active.".into());
        }
        if let Ok(ref entries) = result {
            let body = serde_json::to_string(entries).map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO asx_directory(id,body,fetched_at) VALUES(1,?1,strftime('%Y-%m-%dT%H:%M:%fZ','now')) ON CONFLICT(id) DO UPDATE SET body=excluded.body,fetched_at=excluded.fetched_at", [body]).map_err(|e| e.to_string())?;
        }
        tx.execute("UPDATE settings SET market_refresh_id=NULL WHERE id=1", [])
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        result?;
        self.directory(profile_id)
    }
    pub fn begin_refresh(&mut self, profile_id: &str, ids: Vec<i64>) -> Result<RefreshJob, String> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        Self::check_profile(&tx, profile_id)?;
        let mut unique = ids;
        unique.sort_unstable();
        unique.dedup();
        if unique.is_empty() || unique.len() > 20 {
            return Err("Choose between 1 and 20 securities to refresh.".into());
        }
        let mut securities = Vec::new();
        for id in unique {
            let security = tx
                .query_row(
                    "SELECT ticker,price_version FROM securities WHERE id=?1",
                    [id],
                    |r| {
                        Ok(RefreshSecurity {
                            id,
                            ticker: r.get(0)?,
                            price_version: r.get(1)?,
                        })
                    },
                )
                .optional()
                .map_err(|e| e.to_string())?
                .ok_or("Security not found.")?;
            securities.push(security);
        }
        let id = Self::reserve_market_requests(&tx, securities.len() as i64)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(RefreshJob {
            profile_id: profile_id.into(),
            id,
            securities,
        })
    }
    pub fn finish_refresh(
        &mut self,
        job: RefreshJob,
        results: Vec<Result<Vec<DailyPrice>, String>>,
    ) -> Result<Snapshot, String> {
        if results.len() != job.securities.len() {
            return Err("Incomplete price refresh.".into());
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        Self::check_profile(&tx, &job.profile_id)?;
        let active: Option<String> = tx
            .query_row(
                "SELECT market_refresh_id FROM settings WHERE id=1",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if active.as_deref() != Some(&job.id) {
            return Err("This refresh was replaced. Reload your portfolio.".into());
        }
        for (security, result) in job.securities.iter().zip(results) {
            tx.execute_batch("SAVEPOINT market_symbol")
                .map_err(|e| e.to_string())?;
            let saved = result.and_then(|prices| {
                let latest=prices.last().ok_or("No daily history was returned.")?;
                let (version,old_date):(i64,Option<String>)=tx.query_row("SELECT price_version,price_as_of FROM securities WHERE id=?1",[security.id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
                if version!=security.price_version { return Err("Reference price changed during refresh. Cached prices kept; refresh again later.".into()); }
                if old_date.is_some_and(|date| date>latest.session_date) { return Err("EODHD returned an older session than the cached price. Cached prices kept.".into()); }
                tx.execute("DELETE FROM daily_prices WHERE security_id=?1",[security.id]).map_err(|e|e.to_string())?;
                for p in &prices {
                    tx.execute("INSERT INTO daily_prices(security_id,session_date,close_micros,adjusted_close_micros,open_micros,high_micros,low_micros) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![security.id,p.session_date,p.close_micros,p.adjusted_close_micros,p.open_micros,p.high_micros,p.low_micros]).map_err(|e|e.to_string())?;
                }
                tx.execute("UPDATE securities SET current_price_micros=?1,price_as_of=?2,price_source='eodhd',price_updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'),market_fetched_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'),market_error=NULL,price_version=price_version+1 WHERE id=?3",params![latest.close_micros,latest.session_date,security.id]).map_err(|e|e.to_string())?;
                let (cash,starting):(i64,i64)=tx.query_row("SELECT SUM(amount_micros),SUM(CASE WHEN kind='opening_capital' THEN amount_micros ELSE 0 END) FROM cash_ledger WHERE portfolio_id=1",[],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
                trading::read(&tx,cash,starting)?;
                Ok(())
            });
            if let Err(error) = saved {
                tx.execute_batch("ROLLBACK TO market_symbol")
                    .map_err(|e| e.to_string())?;
                tx.execute(
                    "UPDATE securities SET market_error=?1 WHERE id=?2",
                    params![error, security.id],
                )
                .map_err(|e| e.to_string())?;
            }
            tx.execute_batch("RELEASE market_symbol")
                .map_err(|e| e.to_string())?;
        }
        tx.execute("UPDATE settings SET market_refresh_id=NULL WHERE id=1", [])
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        self.snapshot()?.ok_or("Portfolio unavailable.".into())
    }

    pub fn create_profile(&mut self, input: CreateProfile) -> Result<Snapshot, String> {
        validate_setup(
            &input.display_name,
            input.starting_capital_micros,
            input.default_brokerage_micros,
        )?;
        let tx = self.connection.transaction().map_err(|e| e.to_string())?;
        let exists: bool = tx
            .query_row("SELECT EXISTS(SELECT 1 FROM settings)", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if exists {
            return Err("A local portfolio already exists. It cannot be overwritten.".into());
        }
        tx.execute("INSERT INTO settings(id,display_name,default_brokerage_micros,currency,primary_market,profile_id) VALUES(1,?1,?2,'AUD','ASX',lower(hex(randomblob(16))))",params![input.display_name.trim(),input.default_brokerage_micros]).map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO portfolios(id,name) VALUES(1,'My ASX portfolio')",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO cash_ledger(portfolio_id,kind,amount_micros,description) VALUES(1,'opening_capital',?1,'Starting virtual capital')",[input.starting_capital_micros]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        self.snapshot()?
            .ok_or_else(|| "Portfolio creation did not complete.".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> CreateProfile {
        CreateProfile {
            display_name: " Alex ".into(),
            starting_capital_micros: 1_000_000_000,
            default_brokerage_micros: 3_000_000,
        }
    }
    fn memory() -> Store {
        Store::from_connection(Connection::open_in_memory().unwrap()).unwrap()
    }
    #[test]
    fn v5_close_only_cache_migrates_without_fabricating_candles() {
        let c = Connection::open_in_memory().unwrap();
        for migration in [
            include_str!("../migrations/001_foundation.sql"),
            include_str!("../migrations/002_manual_trading.sql"),
            include_str!("../migrations/003_trading_journal.sql"),
            include_str!("../migrations/004_notes_and_reset.sql"),
            include_str!("../migrations/005_market_data.sql"),
        ] {
            c.execute_batch(migration).unwrap();
        }
        c.execute_batch("INSERT INTO settings(id,display_name,default_brokerage_micros,currency,primary_market) VALUES(1,'Legacy',3000000,'AUD','ASX'); INSERT INTO portfolios(id,name) VALUES(1,'Portfolio'); INSERT INTO cash_ledger(portfolio_id,kind,amount_micros,description) VALUES(1,'opening_capital',1000000000,'Starting capital'); INSERT INTO securities(id,ticker,name,market_fetched_at) VALUES(1,'BHP','BHP Group','2026-10-01T00:00:00Z'); INSERT INTO daily_prices VALUES(1,'2026-09-30',10000000,9900000); UPDATE settings SET profile_id=lower(hex(randomblob(16))); PRAGMA user_version=5;").unwrap();
        let db = Store::from_connection(c).unwrap();
        let snapshot = db.snapshot().unwrap().unwrap();
        assert_eq!(snapshot.cash_micros, 1_000_000_000);
        let price = &snapshot.market.histories[0].prices[0];
        assert_eq!(price.close_micros, 10_000_000);
        assert_eq!(price.open_micros, None);
        assert_eq!(price.high_micros, None);
        assert_eq!(price.low_micros, None);
        assert!(db
            .directory(&snapshot.profile_id)
            .unwrap()
            .entries
            .is_empty());
    }
    #[test]
    fn directory_shares_quota_preserves_cache_and_rejects_reset_races() {
        let mut db = prepared();
        let id = profile(&db);
        let before = db.snapshot().unwrap().unwrap();
        assert!(db.directory(&id).unwrap().entries.is_empty());
        let job = db.begin_directory(&id).unwrap();
        assert!(db.begin_refresh(&id, vec![1]).is_err());
        let entries = crate::directory::parse(
            br#"[{"Code":"BHP","Name":"BHP Group","Type":"Common Stock","Currency":"AUD"}]"#,
        )
        .unwrap();
        db.finish_directory(&id, &job, Ok(entries)).unwrap();
        let after = db.snapshot().unwrap().unwrap();
        assert_eq!(after.cash_micros, before.cash_micros);
        assert_eq!(
            after.trading.securities.len(),
            before.trading.securities.len()
        );
        assert_eq!(after.market.requests_today, 1);
        allow_next_refresh(&db);
        let job = db.begin_directory(&id).unwrap();
        assert!(db
            .finish_directory(&id, &job, Err("Connection failed".into()))
            .is_err());
        assert_eq!(db.directory(&id).unwrap().entries[0].ticker, "BHP");
        assert!(!db.snapshot().unwrap().unwrap().market.refreshing);
        allow_next_refresh(&db);
        let job = db.begin_directory(&id).unwrap();
        db.reset_profile(&id, "RESET").unwrap();
        let fresh = db.create_profile(input()).unwrap();
        assert!(db.finish_directory(&id, &job, Ok(vec![])).is_err());
        assert!(db.directory(&fresh.profile_id).unwrap().entries.is_empty());
        assert_eq!(fresh.market.requests_today, 3);
    }
    #[test]
    fn directory_persists_offline_across_reopen() {
        let path = std::env::temp_dir().join(format!(
            "papertrader-directory-{}.sqlite3",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let id;
        {
            let mut db = Store::open(&path).unwrap();
            id = db.create_profile(input()).unwrap().profile_id;
            let job = db.begin_directory(&id).unwrap();
            let entries = crate::directory::parse(
                br#"[{"Code":"BHP","Name":"BHP Group","Type":"Common Stock","Currency":"AUD"}]"#,
            )
            .unwrap();
            db.finish_directory(&id, &job, Ok(entries)).unwrap();
        }
        let db = Store::open(&path).unwrap();
        assert_eq!(db.directory(&id).unwrap().entries[0].name, "BHP Group");
        drop(db);
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn creates_profile_and_derives_cash_from_ledger() {
        let mut db = memory();
        assert!(db.snapshot().unwrap().is_none());
        let result = db.create_profile(input()).unwrap();
        assert_eq!(result.display_name, "Alex");
        assert_eq!(result.cash_micros, 1_000_000_000);
        assert_eq!(result.starting_capital_micros, result.cash_micros);
        assert_eq!(result.default_brokerage_micros, 3_000_000);
        assert!(db.create_profile(input()).is_err());
        let count: i64 = db
            .connection
            .query_row("SELECT COUNT(*) FROM cash_ledger", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
    #[test]
    fn rejects_invalid_setup_without_partial_records() {
        let mut db = memory();
        let mut bad = input();
        bad.starting_capital_micros = -1;
        assert!(db.create_profile(bad).is_err());
        assert!(db.snapshot().unwrap().is_none());
        assert!(db.create_profile(input()).is_ok());
    }
    #[test]
    fn transaction_rolls_back_when_ledger_write_fails() {
        let mut db = memory();
        db.connection.execute_batch("CREATE TRIGGER fail_insert BEFORE INSERT ON cash_ledger BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
        assert!(db.create_profile(input()).is_err());
        assert!(db.snapshot().unwrap().is_none());
        let count: i64 = db
            .connection
            .query_row("SELECT COUNT(*) FROM portfolios", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
    #[test]
    fn ledger_is_immutable_and_enforces_foreign_keys() {
        let mut db = memory();
        db.create_profile(input()).unwrap();
        assert!(db
            .connection
            .execute("UPDATE cash_ledger SET amount_micros=1", [])
            .is_err());
        assert!(db
            .connection
            .execute("DELETE FROM cash_ledger", [])
            .is_err());
        assert!(db.connection.execute("INSERT INTO cash_ledger(portfolio_id,kind,amount_micros,description) VALUES(2,'opening_capital',10000,'invalid')",[]).is_err());
    }
    #[test]
    fn persists_across_reopen_and_migration_is_repeatable() {
        let path =
            std::env::temp_dir().join(format!("papertrader-test-{}.sqlite", std::process::id()));
        {
            let mut db = Store::open(&path).unwrap();
            db.create_profile(input()).unwrap();
        }
        {
            let db = Store::open(&path).unwrap();
            assert_eq!(db.snapshot().unwrap().unwrap().cash_micros, 1_000_000_000);
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn refuses_newer_schema() {
        let c = Connection::open_in_memory().unwrap();
        c.pragma_update(None, "user_version", 8).unwrap();
        assert!(Store::from_connection(c).is_err());
    }
    fn prepared() -> Store {
        let mut db = memory();
        db.create_profile(input()).unwrap();
        db.create_security(CreateSecurity {
            ticker: "ben".into(),
            name: "Bendigo and Adelaide Bank".into(),
        })
        .unwrap();
        db
    }
    fn fill(
        n: u64,
        side: crate::engine::Side,
        quantity: i64,
        price: i64,
        brokerage: i64,
    ) -> ExecuteTrade {
        ExecuteTrade {
            request_id: format!("00000000-0000-4000-8000-{n:012x}"),
            security_id: 1,
            side,
            quantity,
            price_micros: price,
            brokerage_micros: brokerage,
            journal: None,
        }
    }
    #[test]
    fn ben_buy_partial_sell_and_close_reconcile_cash_and_pnl() {
        use crate::engine::Side::*;
        let mut db = prepared();
        let s = db
            .execute_trade(fill(1, BUY, 24, 10_115_000, 3_000_000))
            .unwrap();
        assert_eq!(s.cash_micros, 754_240_000);
        assert_eq!(s.trading.positions[0].quantity, 24);
        assert_eq!(s.trading.positions[0].cost_basis_micros, 245_760_000);
        assert_eq!(s.trading.positions[0].average_entry_micros, 10_115_000);
        assert_eq!(s.trading.portfolio_value_micros, None);
        let s = db
            .set_price(SetPrice {
                security_id: 1,
                price_micros: 10_600_000,
            })
            .unwrap();
        assert_eq!(s.trading.portfolio_value_micros, Some(1_008_640_000));
        assert_eq!(s.trading.unrealised_pnl_micros, Some(8_640_000));
        assert!(s.trading.positions[0].price_updated_at.is_some());
        let s = db
            .execute_trade(fill(2, SELL, 12, 10_600_000, 3_000_000))
            .unwrap();
        assert_eq!(s.cash_micros, 878_440_000);
        assert_eq!(s.trading.realised_pnl_micros, 1_320_000);
        assert_eq!(s.trading.capital_invested_micros, 122_880_000);
        assert_eq!(s.trading.unrealised_pnl_micros, Some(4_320_000));
        assert_eq!(
            s.trading.total_return_micros,
            Some(s.trading.realised_pnl_micros + s.trading.unrealised_pnl_micros.unwrap())
        );
        let s = db
            .execute_trade(fill(3, SELL, 12, 9_950_000, 3_000_000))
            .unwrap();
        assert_eq!(s.cash_micros, 994_840_000);
        assert!(s.trading.positions.is_empty());
        assert_eq!(s.trading.realised_pnl_micros, -5_160_000);
        assert_eq!(s.trading.brokerage_paid_micros, 9_000_000);
        assert_eq!(s.trading.total_return_micros, Some(-5_160_000));
        assert_eq!(s.trading.executions.len(), 3);
    }
    #[test]
    fn rejects_overselling_insufficient_cash_and_invalid_trade_without_writes() {
        use crate::engine::Side::*;
        let mut db = prepared();
        for order in [
            fill(1, SELL, 1, 10_000_000, 0),
            fill(2, BUY, 100, 10_000_000, 3_000_000),
            fill(3, BUY, 0, 10_000_000, 0),
            fill(4, BUY, 1, 0, 0),
            fill(5, BUY, 1, 10_000_000, -1),
        ] {
            assert!(db.execute_trade(order).is_err());
        }
        let s = db.snapshot().unwrap().unwrap();
        assert!(s.trading.executions.is_empty());
        assert_eq!(s.cash_micros, 1_000_000_000);
        db.execute_trade(fill(6, BUY, 1, 10_000_000, 0)).unwrap();
        assert!(db.execute_trade(fill(7, SELL, 2, 10_000_000, 0)).is_err());
        assert_eq!(db.snapshot().unwrap().unwrap().trading.executions.len(), 1);
    }
    #[test]
    fn duplicate_requests_are_idempotent_and_conflicting_reuse_is_rejected() {
        use crate::engine::Side::*;
        let mut db = prepared();
        db.execute_trade(fill(1, BUY, 24, 10_115_000, 3_000_000))
            .unwrap();
        let s = db
            .execute_trade(fill(1, BUY, 24, 10_115_000, 3_000_000))
            .unwrap();
        assert_eq!(s.cash_micros, 754_240_000);
        assert_eq!(s.trading.executions.len(), 1);
        assert!(db
            .execute_trade(fill(1, BUY, 25, 10_115_000, 3_000_000))
            .is_err());
    }
    #[test]
    fn failed_cash_write_rolls_back_execution_and_executions_are_immutable() {
        use crate::engine::Side::*;
        let mut db = prepared();
        db.connection.execute_batch("CREATE TRIGGER fail_trade BEFORE INSERT ON cash_ledger WHEN NEW.kind='BUY' BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
        assert!(db.execute_trade(fill(1, BUY, 1, 10_000_000, 0)).is_err());
        let count: i64 = db
            .connection
            .query_row("SELECT COUNT(*) FROM executions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
        db.connection
            .execute_batch("DROP TRIGGER fail_trade;")
            .unwrap();
        db.execute_trade(fill(1, BUY, 1, 10_000_000, 0)).unwrap();
        assert!(db
            .connection
            .execute("UPDATE executions SET price_micros=1", [])
            .is_err());
        assert!(db.connection.execute("DELETE FROM executions", []).is_err());
        assert!(db
            .connection
            .execute(
                "UPDATE cash_ledger SET amount_micros=1 WHERE execution_id IS NOT NULL",
                []
            )
            .is_err());
    }
    #[test]
    fn price_and_security_validation_and_small_mark_are_safe() {
        use crate::engine::Side::*;
        let mut db = prepared();
        assert!(db
            .create_security(CreateSecurity {
                ticker: "BEN".into(),
                name: "Duplicate".into()
            })
            .is_err());
        assert!(db
            .create_security(CreateSecurity {
                ticker: "bad/ticker".into(),
                name: "Invalid".into()
            })
            .is_err());
        assert!(db
            .set_price(SetPrice {
                security_id: 1,
                price_micros: 0
            })
            .is_err());
        assert!(db
            .set_price(SetPrice {
                security_id: 99,
                price_micros: 10_000_000
            })
            .is_err());
        db.execute_trade(fill(1, BUY, 1, 1_000_000, 0)).unwrap();
        let s = db
            .set_price(SetPrice {
                security_id: 1,
                price_micros: 1,
            })
            .unwrap();
        assert_eq!(s.trading.positions[0].value_micros, Some(0));
        assert_eq!(s.trading.unrealised_pnl_micros, Some(-1_000_000));
    }
    #[test]
    fn migration_preserves_v1_profile_and_opening_ledger_without_seeding_trades() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(include_str!("../migrations/001_foundation.sql"))
            .unwrap();
        c.execute_batch("INSERT INTO settings(id,display_name,default_brokerage_micros,currency,primary_market) VALUES(1,'Legacy',3000000,'AUD','ASX'); INSERT INTO portfolios(id,name) VALUES(1,'My ASX portfolio'); INSERT INTO cash_ledger(id,portfolio_id,kind,amount_micros,description) VALUES(42,1,'opening_capital',1000000000,'Starting virtual capital'); PRAGMA user_version=1;").unwrap();
        let db = Store::from_connection(c).unwrap();
        let s = db.snapshot().unwrap().unwrap();
        assert_eq!(s.display_name, "Legacy");
        assert_eq!(s.cash_micros, 1_000_000_000);
        assert!(s.trading.securities.is_empty());
        assert!(s.trading.executions.is_empty());
        let id: i64 = db
            .connection
            .query_row("SELECT id FROM cash_ledger", [], |r| r.get(0))
            .unwrap();
        assert_eq!(id, 42);
    }
    fn database_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "papertrader-{label}-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
    #[test]
    fn executions_prices_and_positions_persist_across_restart() {
        use crate::engine::Side::*;
        let path = database_path("restart");
        {
            let mut db = Store::open(&path).unwrap();
            db.create_profile(input()).unwrap();
            db.create_security(CreateSecurity {
                ticker: "BEN".into(),
                name: "Bank".into(),
            })
            .unwrap();
            db.execute_trade(fill(1, BUY, 24, 10_115_000, 3_000_000))
                .unwrap();
            db.set_price(SetPrice {
                security_id: 1,
                price_micros: 10_600_000,
            })
            .unwrap();
        }
        {
            let db = Store::open(&path).unwrap();
            let s = db.snapshot().unwrap().unwrap();
            assert_eq!(s.cash_micros, 754_240_000);
            assert_eq!(s.trading.positions[0].quantity, 24);
            assert_eq!(s.trading.executions.len(), 1);
            assert_eq!(s.trading.portfolio_value_micros, Some(1_008_640_000));
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn another_app_instance_cannot_spend_stale_cash() {
        use crate::engine::Side::*;
        let path = database_path("two-apps");
        {
            let mut first = Store::open(&path).unwrap();
            first.create_profile(input()).unwrap();
            first
                .create_security(CreateSecurity {
                    ticker: "BEN".into(),
                    name: "Bank".into(),
                })
                .unwrap();
            let mut second = Store::open(&path).unwrap();
            assert_eq!(
                second.snapshot().unwrap().unwrap().cash_micros,
                1_000_000_000
            );
            first
                .execute_trade(fill(1, BUY, 80, 10_000_000, 3_000_000))
                .unwrap();
            assert!(second
                .execute_trade(fill(2, BUY, 30, 10_000_000, 3_000_000))
                .is_err());
            assert_eq!(second.snapshot().unwrap().unwrap().cash_micros, 197_000_000);
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn multi_security_valuation_and_closed_position_realised_pnl_are_aggregated() {
        use crate::engine::Side::*;
        let mut db = prepared();
        db.create_security(CreateSecurity {
            ticker: "ABC".into(),
            name: "Another company".into(),
        })
        .unwrap();
        db.execute_trade(fill(1, BUY, 10, 10_000_000, 0)).unwrap();
        let mut order = fill(2, BUY, 20, 5_000_000, 0);
        order.security_id = 2;
        db.execute_trade(order).unwrap();
        let s = db
            .set_price(SetPrice {
                security_id: 1,
                price_micros: 11_000_000,
            })
            .unwrap();
        assert_eq!(s.cash_micros, 800_000_000);
        assert_eq!(s.trading.capital_invested_micros, 200_000_000);
        assert_eq!(s.trading.portfolio_value_micros, None);
        let s = db
            .set_price(SetPrice {
                security_id: 2,
                price_micros: 4_000_000,
            })
            .unwrap();
        assert_eq!(s.trading.portfolio_value_micros, Some(990_000_000));
        assert_eq!(s.trading.unrealised_pnl_micros, Some(-10_000_000));
        let s = db.execute_trade(fill(3, SELL, 10, 11_000_000, 0)).unwrap();
        assert_eq!(s.trading.realised_pnl_micros, 10_000_000);
        assert_eq!(s.trading.positions.len(), 1);
        assert_eq!(s.trading.total_return_micros, Some(-10_000_000));
        assert_eq!(
            s.trading.realised_pnl_micros + s.trading.unrealised_pnl_micros.unwrap(),
            -10_000_000
        );
    }
    #[test]
    fn oversized_manual_valuation_is_rejected_without_changing_prior_price() {
        use crate::engine::Side::*;
        let mut db = prepared();
        db.execute_trade(fill(1, BUY, 24, 10_115_000, 3_000_000))
            .unwrap();
        db.set_price(SetPrice {
            security_id: 1,
            price_micros: 10_600_000,
        })
        .unwrap();
        assert!(db
            .set_price(SetPrice {
                security_id: 1,
                price_micros: crate::domain::MAX_MONEY
            })
            .is_err());
        assert_eq!(
            db.snapshot().unwrap().unwrap().trading.securities[0].current_price_micros,
            Some(10_600_000)
        );
    }
    fn entry_plan() -> journal::Content {
        journal::Content {
            thesis: "Support may hold".into(),
            entry_trigger: "Bounce at support".into(),
            target_micros: Some(10_600_000),
            stop_micros: Some(9_950_000),
            planned_risk_micros: Some(10_000_000),
            notes: "Watch volume".into(),
            ..Default::default()
        }
    }
    #[test]
    fn journal_records_entry_with_fill_and_keeps_original_after_edits() {
        use crate::engine::Side::*;
        let mut db = prepared();
        let mut buy = fill(1, BUY, 24, 10_115_000, 3_000_000);
        buy.journal = Some(entry_plan());
        let s = db.execute_trade(buy).unwrap();
        let entry = &s.journal[0].fills[0];
        assert!(entry.captured_with_fill);
        assert_eq!(entry.revisions[0].content, entry_plan());
        assert_eq!(s.journal[0].id, 1);
        let mut amended = entry_plan();
        amended.thesis = "Updated explanation".into();
        let s = db
            .save_journal(SaveJournal {
                execution_id: 1,
                expected_version: 1,
                content: amended.clone(),
            })
            .unwrap();
        assert_eq!(s.cash_micros, 754_240_000);
        assert_eq!(s.trading.executions[0].price_micros, 10_115_000);
        let revisions = &s.journal[0].fills[0].revisions;
        assert_eq!(revisions.len(), 2);
        assert_eq!(revisions[0].content.thesis, "Support may hold");
        assert_eq!(revisions[1].content.thesis, "Updated explanation");
        assert!(db
            .save_journal(SaveJournal {
                execution_id: 1,
                expected_version: 1,
                content: entry_plan()
            })
            .is_err());
        let s = db
            .save_journal(SaveJournal {
                execution_id: 1,
                expected_version: 2,
                content: amended,
            })
            .unwrap();
        assert_eq!(s.journal[0].fills[0].revisions.len(), 2);
        assert!(db
            .connection
            .execute("UPDATE journal_revisions SET thesis='rewritten'", [])
            .is_err());
        assert!(db
            .connection
            .execute("DELETE FROM journal_entries", [])
            .is_err());
    }
    #[test]
    fn journal_partial_and_full_exit_reconcile_gross_fees_net_and_link_review() {
        use crate::engine::Side::*;
        let mut db = prepared();
        db.execute_trade(fill(1, BUY, 24, 10_115_000, 3_000_000))
            .unwrap();
        let mut sell = fill(2, SELL, 12, 10_600_000, 3_000_000);
        sell.journal = Some(journal::Content {
            exit_reason: "Reached target".into(),
            followed_plan: Some(true),
            went_well: "Kept position size small".into(),
            went_poorly: "Entered early".into(),
            would_change: "Wait for confirmation".into(),
            ..Default::default()
        });
        let s = db.execute_trade(sell).unwrap();
        let t = &s.journal[0];
        assert!(t.closed_at.is_none());
        assert_eq!(t.quantity_open, 12);
        assert_eq!(t.gross_pnl_micros, 5_820_000);
        assert_eq!(t.realised_costs_micros, 4_500_000);
        assert_eq!(t.net_pnl_micros, 1_320_000);
        assert_eq!(t.brokerage_paid_micros, 6_000_000);
        assert_eq!(t.fills[1].revisions[0].content.followed_plan, Some(true));
        assert_eq!(t.fills[1].released_cost_micros, Some(122_880_000));
        let s = db
            .execute_trade(fill(3, SELL, 12, 9_950_000, 3_000_000))
            .unwrap();
        let t = &s.journal[0];
        assert!(t.closed_at.is_some());
        assert_eq!(t.quantity_open, 0);
        assert_eq!(t.gross_pnl_micros, 3_840_000);
        assert_eq!(t.realised_costs_micros, 9_000_000);
        assert_eq!(t.net_pnl_micros, -5_160_000);
        assert_eq!(t.realised_basis_micros, 245_760_000);
        assert_eq!(t.net_pnl_micros, s.trading.realised_pnl_micros);
    }
    #[test]
    fn journal_groups_additional_buys_partial_sells_and_starts_new_after_close() {
        use crate::engine::Side::*;
        let mut db = prepared();
        db.execute_trade(fill(1, BUY, 10, 10_000_000, 3_000_000))
            .unwrap();
        db.execute_trade(fill(2, BUY, 10, 12_000_000, 3_000_000))
            .unwrap();
        let s = db
            .execute_trade(fill(3, SELL, 5, 15_000_000, 3_000_000))
            .unwrap();
        assert_eq!(s.journal.len(), 1);
        assert_eq!(s.journal[0].quantity_bought, 20);
        assert_eq!(s.journal[0].fills.len(), 3);
        assert_eq!(s.journal[0].net_pnl_micros, 15_500_000);
        db.execute_trade(fill(4, SELL, 15, 9_000_000, 3_000_000))
            .unwrap();
        let s = db
            .execute_trade(fill(5, BUY, 2, 8_000_000, 3_000_000))
            .unwrap();
        assert_eq!(s.journal.len(), 2);
        assert_eq!(s.journal[0].net_pnl_micros, -22_000_000);
        assert_eq!(s.journal[1].id, 5);
        assert_eq!(s.journal[1].entry_cost_micros, 19_000_000);
        assert_eq!(s.journal[1].net_pnl_micros, 0);
    }
    #[test]
    fn failed_journal_write_rolls_back_cash_and_execution_and_bad_content_is_rejected() {
        use crate::engine::Side::*;
        let mut db = prepared();
        db.connection.execute_batch("CREATE TRIGGER fail_journal BEFORE INSERT ON journal_revisions BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
        assert!(db.execute_trade(fill(1, BUY, 1, 10_000_000, 0)).is_err());
        let s = db.snapshot().unwrap().unwrap();
        assert_eq!(s.cash_micros, 1_000_000_000);
        assert!(s.journal.is_empty());
        assert!(s.trading.executions.is_empty());
        db.connection
            .execute_batch("DROP TRIGGER fail_journal;")
            .unwrap();
        let mut buy = fill(1, BUY, 1, 10_000_000, 0);
        buy.journal = Some(journal::Content {
            planned_risk_micros: Some(-1),
            ..Default::default()
        });
        assert!(db.execute_trade(buy).is_err());
        db.execute_trade(fill(1, BUY, 1, 10_000_000, 0)).unwrap();
        assert!(db
            .save_journal(SaveJournal {
                execution_id: 1,
                expected_version: 1,
                content: journal::Content {
                    exit_reason: "Wrong side".into(),
                    ..Default::default()
                }
            })
            .is_err());
        assert!(db
            .save_journal(SaveJournal {
                execution_id: 99,
                expected_version: 1,
                content: entry_plan()
            })
            .is_err());
        assert!(db
            .save_journal(SaveJournal {
                execution_id: 1,
                expected_version: 1,
                content: journal::Content {
                    thesis: "a".repeat(4001),
                    ..Default::default()
                }
            })
            .is_err());
    }
    #[test]
    fn v2_migration_backfills_blank_journals_and_preserves_fill_and_cash() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(include_str!("../migrations/001_foundation.sql"))
            .unwrap();
        c.execute_batch(include_str!("../migrations/002_manual_trading.sql"))
            .unwrap();
        c.execute_batch("INSERT INTO settings(id,display_name,default_brokerage_micros,currency,primary_market) VALUES(1,'Legacy',3000000,'AUD','ASX'); INSERT INTO portfolios(id,name) VALUES(1,'Portfolio'); INSERT INTO cash_ledger(portfolio_id,kind,amount_micros,description) VALUES(1,'opening_capital',1000000000,'Starting capital'); INSERT INTO securities(id,ticker,name) VALUES(1,'BEN','Bank'); INSERT INTO executions(id,request_id,portfolio_id,security_id,side,quantity,price_micros,brokerage_micros,notional_micros) VALUES(42,'legacy',1,1,'BUY',24,10115000,3000000,242760000); INSERT INTO cash_ledger(portfolio_id,kind,amount_micros,execution_id,description) VALUES(1,'BUY',-245760000,42,'Manual buy'); PRAGMA user_version=2;").unwrap();
        let mut db = Store::from_connection(c).unwrap();
        let s = db.snapshot().unwrap().unwrap();
        assert_eq!(s.cash_micros, 754_240_000);
        assert_eq!(s.trading.executions[0].id, 42);
        let entry = &s.journal[0].fills[0];
        assert!(!entry.captured_with_fill);
        assert_eq!(entry.revisions[0].content, journal::Content::default());
        let s = db
            .save_journal(SaveJournal {
                execution_id: 42,
                expected_version: 1,
                content: entry_plan(),
            })
            .unwrap();
        assert_eq!(s.journal[0].fills[0].revisions.len(), 2);
        assert_eq!(s.cash_micros, 754_240_000);
    }
    #[test]
    fn journal_and_versions_persist_after_restart_and_trade_retries_compare_original() {
        use crate::engine::Side::*;
        let path = database_path("journal");
        {
            let mut db = Store::open(&path).unwrap();
            db.create_profile(input()).unwrap();
            db.create_security(CreateSecurity {
                ticker: "BEN".into(),
                name: "Bank".into(),
            })
            .unwrap();
            let mut buy = fill(1, BUY, 24, 10_115_000, 3_000_000);
            buy.journal = Some(entry_plan());
            db.execute_trade(buy).unwrap();
            let mut changed = entry_plan();
            changed.notes = "Review later".into();
            db.save_journal(SaveJournal {
                execution_id: 1,
                expected_version: 1,
                content: changed,
            })
            .unwrap();
        }
        {
            let mut db = Store::open(&path).unwrap();
            let mut retry = fill(1, BUY, 24, 10_115_000, 3_000_000);
            retry.journal = Some(entry_plan());
            let s = db.execute_trade(retry).unwrap();
            assert_eq!(s.journal[0].fills[0].revisions.len(), 2);
            assert_eq!(s.trading.executions.len(), 1);
            assert_eq!(
                s.journal[0].fills[0].revisions[1].content.notes,
                "Review later"
            );
            let mut conflict = fill(1, BUY, 24, 10_115_000, 3_000_000);
            conflict.journal = Some(journal::Content {
                thesis: "Different plan".into(),
                ..Default::default()
            });
            assert!(db.execute_trade(conflict).is_err());
        }
        std::fs::remove_file(path).unwrap();
    }
    fn profile(db: &Store) -> String {
        db.snapshot().unwrap().unwrap().profile_id
    }
    fn new_note(title: &str, body: &str) -> SaveNote {
        SaveNote {
            id: None,
            expected_version: None,
            title: title.into(),
            body: body.into(),
        }
    }
    #[test]
    fn notes_create_edit_searchable_text_versions_and_delete_without_financial_changes() {
        let mut db = prepared();
        let id = profile(&db);
        let s = db
            .apply_profile_action(
                &id,
                ProfileAction::SaveNote(new_note(" Watchlist ", "ASX ideas\nKeep notes local")),
            )
            .unwrap();
        assert_eq!(s.notes[0].title, "Watchlist");
        assert_eq!(s.notes[0].version, 1);
        assert_eq!(s.cash_micros, 1_000_000_000);
        let note_id = s.notes[0].id;
        let s = db
            .apply_profile_action(
                &id,
                ProfileAction::SaveNote(SaveNote {
                    id: Some(note_id),
                    expected_version: Some(1),
                    title: "Watchlist".into(),
                    body: "Updated notes".into(),
                }),
            )
            .unwrap();
        assert_eq!(s.notes[0].version, 2);
        assert!(db
            .apply_profile_action(
                &id,
                ProfileAction::SaveNote(SaveNote {
                    id: Some(note_id),
                    expected_version: Some(1),
                    title: "Stale".into(),
                    body: "Old draft".into()
                })
            )
            .is_err());
        assert!(db
            .apply_profile_action(
                &id,
                ProfileAction::DeleteNote(DeleteNote {
                    id: note_id,
                    expected_version: 1
                })
            )
            .is_err());
        let s = db
            .apply_profile_action(
                &id,
                ProfileAction::DeleteNote(DeleteNote {
                    id: note_id,
                    expected_version: 2,
                }),
            )
            .unwrap();
        assert!(s.notes.is_empty());
        assert!(s.trading.executions.is_empty());
    }
    #[test]
    fn notes_validate_content_and_persist_after_restart() {
        let path = database_path("notes");
        {
            let mut db = Store::open(&path).unwrap();
            db.create_profile(input()).unwrap();
            let id = profile(&db);
            assert!(db
                .apply_profile_action(&id, ProfileAction::SaveNote(new_note(" ", "blank title")))
                .is_err());
            assert!(db
                .apply_profile_action(
                    &id,
                    ProfileAction::SaveNote(new_note("Valid", &"x".repeat(50001)))
                )
                .is_err());
            db.apply_profile_action(
                &id,
                ProfileAction::SaveNote(new_note("My research", "A question to revisit")),
            )
            .unwrap();
        }
        {
            let db = Store::open(&path).unwrap();
            let s = db.snapshot().unwrap().unwrap();
            assert_eq!(s.notes.len(), 1);
            assert_eq!(s.notes[0].body, "A question to revisit");
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn reset_requires_exact_confirmation_clears_all_progress_and_recreates_fresh_profile() {
        use crate::engine::Side::*;
        let mut db = prepared();
        let old = profile(&db);
        db.execute_trade(fill(1, BUY, 24, 10_115_000, 3_000_000))
            .unwrap();
        db.save_journal(SaveJournal {
            execution_id: 1,
            expected_version: 1,
            content: entry_plan(),
        })
        .unwrap();
        db.apply_profile_action(
            &old,
            ProfileAction::SaveNote(new_note("Learning", "Keep a plan")),
        )
        .unwrap();
        assert!(db.reset_profile(&old, "reset").is_err());
        assert!(db.reset_profile("wrong-profile", "RESET").is_err());
        assert_eq!(db.snapshot().unwrap().unwrap().cash_micros, 754_240_000);
        db.reset_profile(&old, "RESET").unwrap();
        assert!(db.snapshot().unwrap().is_none());
        for table in [
            "settings",
            "portfolios",
            "cash_ledger",
            "securities",
            "executions",
            "journal_entries",
            "journal_revisions",
            "notes",
        ] {
            let count: i64 = db
                .connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 0);
        }
        let mut fresh = input();
        fresh.display_name = "Fresh trader".into();
        fresh.starting_capital_micros = 5_000_000_000;
        let s = db.create_profile(fresh).unwrap();
        assert_ne!(s.profile_id, old);
        assert_eq!(s.cash_micros, 5_000_000_000);
        assert!(s.notes.is_empty());
        assert!(s.journal.is_empty());
        assert!(s.trading.securities.is_empty());
        db.create_security(CreateSecurity {
            ticker: "BEN".into(),
            name: "Bank".into(),
        })
        .unwrap();
        assert!(db
            .apply_profile_action(
                &old,
                ProfileAction::ExecuteTrade(fill(2, BUY, 1, 10_000_000, 0))
            )
            .is_err());
        assert!(db
            .apply_profile_action(
                &old,
                ProfileAction::SaveNote(new_note("Stale", "Cannot cross resets"))
            )
            .is_err());
        let new = profile(&db);
        db.apply_profile_action(
            &new,
            ProfileAction::ExecuteTrade(fill(3, BUY, 1, 10_000_000, 0)),
        )
        .unwrap();
        assert!(db.connection.execute("DELETE FROM executions", []).is_err());
        assert!(db
            .connection
            .execute("DELETE FROM cash_ledger", [])
            .is_err());
    }
    #[test]
    fn reset_is_persistent_across_restart() {
        let path = database_path("reset");
        {
            let mut db = Store::open(&path).unwrap();
            db.create_profile(input()).unwrap();
            let id = profile(&db);
            db.apply_profile_action(&id, ProfileAction::SaveNote(new_note("Old", "Progress")))
                .unwrap();
            db.reset_profile(&id, "RESET").unwrap();
        }
        {
            let mut db = Store::open(&path).unwrap();
            assert!(db.snapshot().unwrap().is_none());
            let s = db.create_profile(input()).unwrap();
            assert_eq!(s.cash_micros, 1_000_000_000);
            assert!(s.notes.is_empty());
        }
        std::fs::remove_file(path).unwrap();
    }
    fn daily(date: &str, close: i64) -> Vec<DailyPrice> {
        vec![DailyPrice {
            session_date: date.into(),
            close_micros: close,
            open_micros: Some(close),
            high_micros: Some(close + 1),
            low_micros: Some(close - 1),
            adjusted_close_micros: Some(close - 1000),
        }]
    }
    fn allow_next_refresh(db: &Store) {
        db.connection
            .execute(
                "UPDATE settings SET market_refresh_started=unixepoch()-61",
                [],
            )
            .unwrap();
    }
    #[test]
    fn market_refresh_preserves_fills_cash_journal_and_tracks_session_and_source() {
        let mut db = prepared();
        let id = profile(&db);
        db.execute_trade(fill(1, crate::engine::Side::BUY, 24, 10_115_000, 3_000_000))
            .unwrap();
        let before = db.snapshot().unwrap().unwrap();
        let job = db.begin_refresh(&id, vec![1, 1]).unwrap();
        assert_eq!(job.securities.len(), 1);
        assert!(db.snapshot().unwrap().unwrap().market.refreshing);
        assert!(db.begin_refresh(&id, vec![1]).is_err());
        let s = db
            .finish_refresh(job, vec![Ok(daily("2026-09-30", 10_600_000))])
            .unwrap();
        assert_eq!(s.cash_micros, before.cash_micros);
        assert_eq!(s.trading.executions[0].price_micros, 10_115_000);
        assert_eq!(
            s.journal[0].fills[0].revisions.len(),
            before.journal[0].fills[0].revisions.len()
        );
        assert_eq!(s.trading.unrealised_pnl_micros, Some(8_640_000));
        assert_eq!(s.trading.securities[0].price_source, "eodhd");
        assert_eq!(
            s.trading.securities[0].price_as_of.as_deref(),
            Some("2026-09-30")
        );
        assert_eq!(s.market.requests_today, 1);
        assert!(!s.market.refreshing);
        assert_eq!(s.market.histories[0].prices[0].close_micros, 10_600_000);
        assert!(db.begin_refresh(&id, vec![1]).is_err());
    }
    #[test]
    fn failed_and_older_quotes_preserve_cache_and_partial_batch_succeeds() {
        let mut db = prepared();
        let id = profile(&db);
        let j = db.begin_refresh(&id, vec![1]).unwrap();
        db.finish_refresh(j, vec![Ok(daily("2026-09-30", 10_600_000))])
            .unwrap();
        allow_next_refresh(&db);
        db.create_security(CreateSecurity {
            ticker: "BHP".into(),
            name: "BHP".into(),
        })
        .unwrap();
        let j = db.begin_refresh(&id, vec![1, 2]).unwrap();
        let s = db
            .finish_refresh(
                j,
                vec![
                    Err("Offline. Cached prices kept.".into()),
                    Ok(daily("2026-09-30", 40_000_000)),
                ],
            )
            .unwrap();
        let ben = s.trading.securities.iter().find(|s| s.id == 1).unwrap();
        assert_eq!(ben.current_price_micros, Some(10_600_000));
        assert!(ben.market_error.is_some());
        assert_eq!(s.market.histories.len(), 2);
        allow_next_refresh(&db);
        let j = db.begin_refresh(&id, vec![1]).unwrap();
        let s = db
            .finish_refresh(j, vec![Ok(daily("2026-09-29", 11_000_000))])
            .unwrap();
        assert!(s
            .trading
            .securities
            .iter()
            .find(|s| s.id == 1)
            .unwrap()
            .market_error
            .as_ref()
            .unwrap()
            .contains("older"));
    }
    #[test]
    fn market_limits_survive_reset_and_invalid_requests_cost_nothing() {
        let mut db = prepared();
        let id = profile(&db);
        assert!(db.begin_refresh(&id, vec![]).is_err());
        assert!(db.begin_refresh(&id, vec![99]).is_err());
        assert!(db.begin_refresh("stale", vec![1]).is_err());
        assert_eq!(db.snapshot().unwrap().unwrap().market.requests_today, 0);
        db.connection
            .execute("INSERT INTO market_usage VALUES(date('now'),19)", [])
            .unwrap();
        let j = db.begin_refresh(&id, vec![1]).unwrap();
        db.finish_refresh(j, vec![Err("Rate limited".into())])
            .unwrap();
        allow_next_refresh(&db);
        assert!(db.begin_refresh(&id, vec![1]).is_err());
        db.reset_profile(&id, "RESET").unwrap();
        let s = db.create_profile(input()).unwrap();
        assert_eq!(s.market.requests_today, 20);
        assert!(s.market.histories.is_empty());
        db.connection
            .execute("UPDATE market_usage SET utc_day=date('now','-1 day')", [])
            .unwrap();
        assert_eq!(db.snapshot().unwrap().unwrap().market.requests_today, 0);
    }
    #[test]
    fn reset_and_manual_price_changes_reject_inflight_market_writes() {
        let mut db = prepared();
        let id = profile(&db);
        let j = db.begin_refresh(&id, vec![1]).unwrap();
        db.set_price(SetPrice {
            security_id: 1,
            price_micros: 12_000_000,
        })
        .unwrap();
        let s = db
            .finish_refresh(j, vec![Ok(daily("2026-09-30", 10_600_000))])
            .unwrap();
        assert_eq!(
            s.trading.securities[0].current_price_micros,
            Some(12_000_000)
        );
        assert_eq!(s.trading.securities[0].price_source, "manual");
        assert!(s.market.histories.is_empty());
        allow_next_refresh(&db);
        let j = db.begin_refresh(&id, vec![1]).unwrap();
        db.reset_profile(&id, "RESET").unwrap();
        db.create_profile(input()).unwrap();
        db.create_security(CreateSecurity {
            ticker: "BEN".into(),
            name: "Bank".into(),
        })
        .unwrap();
        assert!(db
            .finish_refresh(j, vec![Ok(daily("2026-09-30", 10_600_000))])
            .is_err());
        assert!(db.snapshot().unwrap().unwrap().market.histories.is_empty());
    }
    #[test]
    fn market_history_persists_and_bad_valuation_rolls_back_only_that_symbol() {
        let path = database_path("market");
        {
            let mut db = Store::open(&path).unwrap();
            db.create_profile(input()).unwrap();
            db.create_security(CreateSecurity {
                ticker: "BEN".into(),
                name: "Bank".into(),
            })
            .unwrap();
            db.execute_trade(fill(1, crate::engine::Side::BUY, 24, 10_115_000, 3_000_000))
                .unwrap();
            let id = profile(&db);
            let j = db.begin_refresh(&id, vec![1]).unwrap();
            db.finish_refresh(j, vec![Ok(daily("2026-09-30", 10_600_000))])
                .unwrap();
            allow_next_refresh(&db);
            let j = db.begin_refresh(&id, vec![1]).unwrap();
            let s = db
                .finish_refresh(j, vec![Ok(daily("2026-10-01", crate::domain::MAX_MONEY))])
                .unwrap();
            assert_eq!(
                s.trading.securities[0].current_price_micros,
                Some(10_600_000)
            );
            assert_eq!(s.market.histories[0].prices[0].session_date, "2026-09-30");
            assert!(s.trading.securities[0].market_error.is_some());
        }
        let db = Store::open(&path).unwrap();
        let s = db.snapshot().unwrap().unwrap();
        assert_eq!(s.market.histories[0].prices[0].close_micros, 10_600_000);
        assert_eq!(s.market.requests_today, 2);
        drop(db);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn v4_migration_preserves_manual_prices_and_does_not_seed_history() {
        let c = Connection::open_in_memory().unwrap();
        for sql in [
            include_str!("../migrations/001_foundation.sql"),
            include_str!("../migrations/002_manual_trading.sql"),
            include_str!("../migrations/003_trading_journal.sql"),
            include_str!("../migrations/004_notes_and_reset.sql"),
        ] {
            c.execute_batch(sql).unwrap();
        }
        c.pragma_update(None, "user_version", 4).unwrap();
        c.execute("INSERT INTO settings(id,display_name,default_brokerage_micros,currency,primary_market,profile_id) VALUES(1,'Trader',0,'AUD','ASX',lower(hex(randomblob(16))))",[]).unwrap();
        c.execute(
            "INSERT INTO portfolios(id,name) VALUES(1,'Paper portfolio')",
            [],
        )
        .unwrap();
        c.execute("INSERT INTO cash_ledger(portfolio_id,kind,amount_micros,description) VALUES(1,'opening_capital',1000000000,'Capital')",[]).unwrap();
        c.execute("INSERT INTO securities(ticker,name,current_price_micros,price_updated_at) VALUES('BEN','Bank',10115000,'2026-09-30T00:00:00Z')",[]).unwrap();
        let db = Store::from_connection(c).unwrap();
        let s = db.snapshot().unwrap().unwrap();
        assert_eq!(
            s.trading.securities[0].current_price_micros,
            Some(10_115_000)
        );
        assert_eq!(s.trading.securities[0].price_source, "manual");
        assert!(s.market.histories.is_empty());
        assert_eq!(s.market.requests_today, 0);
    }
}
