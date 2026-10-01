use crate::domain::validate_setup;
use crate::journal::{self, JournalTrade, SaveJournal};
use crate::trading::{self, CreateSecurity, ExecuteTrade, SetPrice, TradingSnapshot};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::path::Path;

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
    pub default_brokerage_micros: i64,
    pub currency: String,
    pub primary_market: String,
    pub starting_capital_micros: i64,
    pub cash_micros: i64,
    pub created_at: String,
    pub trading: TradingSnapshot,
    pub journal: Vec<JournalTrade>,
}
pub struct Store {
    connection: Connection,
}
impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        Self::from_connection(Connection::open(path).map_err(|e| e.to_string())?)
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
        if version > 3 {
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
        Ok(Self { connection })
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
            (SELECT COALESCE(SUM(amount_micros),0) FROM cash_ledger WHERE portfolio_id=1),p.created_at
            FROM settings s JOIN portfolios p ON p.id=s.id WHERE s.id=1", [],
            |r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,i64>(4)?,r.get::<_,i64>(5)?,r.get::<_,String>(6)?))
        ).optional().map_err(|e|e.to_string())?;
        let Some((
            display_name,
            default_brokerage_micros,
            currency,
            primary_market,
            starting_capital_micros,
            cash_micros,
            created_at,
        )) = metadata
        else {
            return Ok(None);
        };
        let trading = trading::read(&tx, cash_micros, starting_capital_micros)?;
        let journal = journal::read(&tx)?;
        Ok(Some(Snapshot {
            display_name,
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
        tx.execute("INSERT INTO settings(id,display_name,default_brokerage_micros,currency,primary_market) VALUES(1,?1,?2,'AUD','ASX')",params![input.display_name.trim(),input.default_brokerage_micros]).map_err(|e| e.to_string())?;
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
        c.pragma_update(None, "user_version", 4).unwrap();
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
}
