//! Virtual cash only. Deposits are contributions, never investment returns.
use crate::{domain::MAX_MONEY, engine::bounded};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deposit {
    pub request_id: String,
    pub account: String,
    pub amount_micros: i64,
    pub description: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositRecord {
    pub id: i64,
    pub account: String,
    pub amount_micros: i64,
    pub description: String,
    pub created_at: String,
}
pub fn cash(c: &Connection) -> Result<i64, String> {
    let stock: i64 = c
        .query_row(
            "SELECT COALESCE(SUM(amount_micros),0) FROM cash_ledger WHERE portfolio_id=1",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let deposits: i64 = c
        .query_row(
            "SELECT COALESCE(SUM(amount_micros),0) FROM virtual_deposits WHERE account='stocks'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let shared:i64=c.query_row("SELECT CASE WHEN EXISTS(SELECT 1 FROM crypto_wallet WHERE mode='shared') THEN COALESCE((SELECT SUM(cash_delta_micros) FROM crypto_executions),0) ELSE 0 END",[],|r|r.get(0)).map_err(|e|e.to_string())?;
    bounded(i128::from(stock) + i128::from(deposits) + i128::from(shared))
}
pub fn stock_contributions(c: &Connection) -> Result<i64, String> {
    let initial: i64 = c
        .query_row(
            "SELECT COALESCE(SUM(amount_micros),0) FROM cash_ledger WHERE kind='opening_capital'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let extra: i64 = c
        .query_row(
            "SELECT COALESCE(SUM(amount_micros),0) FROM virtual_deposits WHERE account='stocks'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    bounded(i128::from(initial) + i128::from(extra))
}
pub fn valid_request(id: &str) -> Result<(), String> {
    if !(16..=80).contains(&id.len()) || !id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') {
        return Err("Invalid request ID.".into());
    }
    Ok(())
}
pub fn deposit(c: &Connection, input: Deposit) -> Result<(), String> {
    valid_request(&input.request_id)?;
    if !["stocks", "crypto"].contains(&input.account.as_str())
        || !(10000..=MAX_MONEY).contains(&input.amount_micros)
        || input.amount_micros % 10000 != 0
        || input.description.chars().count() > 200
    {
        return Err(
            "Enter a positive virtual deposit in whole cents, within the supported limit.".into(),
        );
    }
    if input.account == "crypto"
        && !c
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM crypto_wallet WHERE mode='separate')",
                [],
                |r| r.get::<_, bool>(0),
            )
            .map_err(|e| e.to_string())?
    {
        return Err(
            "Set up a separate crypto wallet first. A shared wallet uses stock deposits.".into(),
        );
    }
    let prior: Option<(String, i64, String)> = c
        .query_row(
            "SELECT account,amount_micros,description FROM virtual_deposits WHERE request_id=?1",
            [&input.request_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some(prior) = prior {
        return if prior == (input.account, input.amount_micros, input.description) {
            Ok(())
        } else {
            Err("This deposit request was already used with different details.".into())
        };
    }
    c.execute("INSERT INTO virtual_deposits(request_id,account,amount_micros,description) VALUES(?1,?2,?3,?4)",params![input.request_id,input.account,input.amount_micros,input.description]).map_err(|e|e.to_string())?;
    cash(c)?;
    stock_contributions(c)?;
    Ok(())
}
pub fn read(c: &Connection) -> Result<Vec<DepositRecord>, String> {
    let mut q=c.prepare("SELECT id,account,amount_micros,description,created_at FROM virtual_deposits ORDER BY id DESC").map_err(|e|e.to_string())?;
    let result = q
        .query_map([], |r| {
            Ok(DepositRecord {
                id: r.get(0)?,
                account: r.get(1)?,
                amount_micros: r.get(2)?,
                description: r.get(3)?,
                created_at: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string());
    result
}
