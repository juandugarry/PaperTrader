use crate::domain::validate_setup;
use rusqlite::{params, Connection, OptionalExtension};
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
        if version > 1 {
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
        Ok(Self { connection })
    }
    pub fn snapshot(&self) -> Result<Option<Snapshot>, String> {
        self.connection.query_row(
            "SELECT s.display_name, s.default_brokerage_micros, s.currency, s.primary_market,
            (SELECT COALESCE(SUM(amount_micros),0) FROM cash_ledger WHERE portfolio_id=1 AND kind='opening_capital'),
            (SELECT COALESCE(SUM(amount_micros),0) FROM cash_ledger WHERE portfolio_id=1), p.created_at
            FROM settings s JOIN portfolios p ON p.id=s.id WHERE s.id=1", [],
            |r| Ok(Snapshot { display_name:r.get(0)?, default_brokerage_micros:r.get(1)?, currency:r.get(2)?, primary_market:r.get(3)?, starting_capital_micros:r.get(4)?, cash_micros:r.get(5)?, created_at:r.get(6)? })
        ).optional().map_err(|e| e.to_string())
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
        c.pragma_update(None, "user_version", 2).unwrap();
        assert!(Store::from_connection(c).is_err());
    }
}
