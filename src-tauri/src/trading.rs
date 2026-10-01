//! SQLite adapter. All fill arithmetic is delegated to the pure engine.
use crate::{
    domain::MAX_MONEY,
    engine::{self, PositionState, Side},
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSecurity {
    pub ticker: String,
    pub name: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPrice {
    pub security_id: i64,
    pub price_micros: i64,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteTrade {
    pub request_id: String,
    pub security_id: i64,
    pub side: Side,
    pub quantity: i64,
    pub price_micros: i64,
    pub brokerage_micros: i64,
    #[serde(default)]
    pub journal: Option<crate::journal::Content>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Security {
    pub id: i64,
    pub ticker: String,
    pub name: String,
    pub current_price_micros: Option<i64>,
    pub price_updated_at: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Execution {
    pub id: i64,
    pub security_id: i64,
    pub ticker: String,
    pub name: String,
    pub side: Side,
    pub quantity: i64,
    pub price_micros: i64,
    pub brokerage_micros: i64,
    pub notional_micros: i64,
    pub cash_delta_micros: i64,
    pub created_at: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub security_id: i64,
    pub ticker: String,
    pub name: String,
    pub quantity: i64,
    pub average_entry_micros: i64,
    pub cost_basis_micros: i64,
    pub current_price_micros: Option<i64>,
    pub price_updated_at: Option<String>,
    pub value_micros: Option<i64>,
    pub unrealised_pnl_micros: Option<i64>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradingSnapshot {
    pub securities: Vec<Security>,
    pub positions: Vec<Position>,
    pub executions: Vec<Execution>,
    pub capital_invested_micros: i64,
    pub realised_pnl_micros: i64,
    pub brokerage_paid_micros: i64,
    pub unrealised_pnl_micros: Option<i64>,
    pub portfolio_value_micros: Option<i64>,
    pub total_return_micros: Option<i64>,
}
fn err(e: rusqlite::Error) -> String {
    e.to_string()
}
pub fn securities(c: &Connection) -> Result<Vec<Security>, String> {
    let mut query=c.prepare("SELECT id,ticker,name,current_price_micros,price_updated_at FROM securities ORDER BY ticker").map_err(err)?;
    let rows = query
        .query_map([], |r| {
            Ok(Security {
                id: r.get(0)?,
                ticker: r.get(1)?,
                name: r.get(2)?,
                current_price_micros: r.get(3)?,
                price_updated_at: r.get(4)?,
            })
        })
        .map_err(err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(err)
}
pub fn executions(c: &Connection) -> Result<Vec<Execution>, String> {
    let mut query=c.prepare("SELECT e.id,e.security_id,s.ticker,s.name,e.side,e.quantity,e.price_micros,e.brokerage_micros,e.notional_micros,l.amount_micros,e.created_at FROM executions e JOIN securities s ON s.id=e.security_id JOIN cash_ledger l ON l.execution_id=e.id WHERE e.portfolio_id=1 ORDER BY e.id").map_err(err)?;
    let rows = query
        .query_map([], |r| {
            let side: String = r.get(4)?;
            Ok(Execution {
                id: r.get(0)?,
                security_id: r.get(1)?,
                ticker: r.get(2)?,
                name: r.get(3)?,
                side: if side == "BUY" { Side::BUY } else { Side::SELL },
                quantity: r.get(5)?,
                price_micros: r.get(6)?,
                brokerage_micros: r.get(7)?,
                notional_micros: r.get(8)?,
                cash_delta_micros: r.get(9)?,
                created_at: r.get(10)?,
            })
        })
        .map_err(err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(err)
}
pub fn states(history: &[Execution]) -> Result<BTreeMap<i64, PositionState>, String> {
    let mut result = BTreeMap::<i64, PositionState>::new();
    for e in history {
        let effect = engine::apply(
            result.entry(e.security_id).or_default(),
            e.side,
            e.quantity,
            e.price_micros,
            e.brokerage_micros,
        )?;
        if effect.notional != e.notional_micros || effect.cash_delta != e.cash_delta_micros {
            return Err("Execution and ledger history disagree.".into());
        }
        result.insert(e.security_id, effect.state);
    }
    Ok(result)
}
pub fn read(c: &Connection, cash: i64, starting: i64) -> Result<TradingSnapshot, String> {
    let securities = securities(c)?;
    let executions = executions(c)?;
    let states = states(&executions)?;
    let mut positions = Vec::new();
    let mut invested = 0_i128;
    let mut realised = 0_i128;
    let mut fees = 0_i128;
    let mut value = Some(0_i128);
    for security in &securities {
        let Some(state) = states.get(&security.id) else {
            continue;
        };
        realised += i128::from(state.realised);
        fees += i128::from(state.brokerage_paid);
        if state.quantity == 0 {
            continue;
        }
        invested += i128::from(state.cost_basis);
        let mark = security
            .current_price_micros
            .map(|p| engine::market_value(state.quantity, p))
            .transpose()?;
        value = value.zip(mark).map(|(sum, mark)| sum + i128::from(mark));
        positions.push(Position {
            security_id: security.id,
            ticker: security.ticker.clone(),
            name: security.name.clone(),
            quantity: state.quantity,
            average_entry_micros: ((i128::from(state.gross_basis) + i128::from(state.quantity) / 2)
                / i128::from(state.quantity)) as i64,
            cost_basis_micros: state.cost_basis,
            current_price_micros: security.current_price_micros,
            price_updated_at: security.price_updated_at.clone(),
            value_micros: mark,
            unrealised_pnl_micros: mark.map(|m| m - state.cost_basis),
        });
    }
    let portfolio_value = value
        .map(|v| engine::bounded(v + i128::from(cash)))
        .transpose()?;
    Ok(TradingSnapshot {
        securities,
        positions,
        executions,
        capital_invested_micros: engine::bounded(invested)?,
        realised_pnl_micros: engine::bounded(realised)?,
        brokerage_paid_micros: engine::bounded(fees)?,
        unrealised_pnl_micros: value.map(|v| engine::bounded(v - invested)).transpose()?,
        portfolio_value_micros: portfolio_value,
        total_return_micros: portfolio_value.map(|v| v - starting),
    })
}
pub fn create_security(c: &Connection, input: CreateSecurity) -> Result<(), String> {
    let ticker = input.ticker.trim().to_ascii_uppercase();
    let name = input.name.trim();
    if ticker.is_empty()
        || ticker.len() > 10
        || !ticker
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return Err("Use an ASX ticker of 1–10 letters or digits.".into());
    }
    if name.is_empty() || name.chars().count() > 120 {
        return Err("Enter a security name of 1–120 characters.".into());
    }
    if c.query_row(
        "SELECT EXISTS(SELECT 1 FROM securities WHERE ticker=?1)",
        [&ticker],
        |r| r.get::<_, bool>(0),
    )
    .map_err(err)?
    {
        return Err("That ticker already exists. Select the existing security.".into());
    }
    c.execute(
        "INSERT INTO securities(ticker,name) VALUES(?1,?2)",
        params![ticker, name],
    )
    .map_err(err)?;
    Ok(())
}
pub fn set_price(c: &Connection, input: SetPrice) -> Result<(), String> {
    if !(1..=MAX_MONEY).contains(&input.price_micros) {
        return Err("Enter a positive manual price within the supported limit.".into());
    }
    if c.execute("UPDATE securities SET current_price_micros=?1,price_updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?2",params![input.price_micros,input.security_id]).map_err(err)?!=1 {return Err("Security not found.".into());}
    Ok(())
}
/// Called inside an IMMEDIATE transaction, preventing two app instances from spending the same cash.
pub fn execute(c: &Connection, input: ExecuteTrade) -> Result<(), String> {
    if input.request_id.len() != 36
        || !input
            .request_id
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == '-')
    {
        return Err("Invalid execution request identifier.".into());
    }
    let previous=c.query_row("SELECT security_id,side,quantity,price_micros,brokerage_micros FROM executions WHERE request_id=?1",[&input.request_id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,r.get::<_,i64>(3)?,r.get::<_,i64>(4)?))).optional().map_err(err)?;
    if let Some(previous) = previous {
        if previous
            == (
                input.security_id,
                input.side.as_str().into(),
                input.quantity,
                input.price_micros,
                input.brokerage_micros,
            )
        {
            let submitted = input.journal.clone().unwrap_or_default();
            let id: i64 = c
                .query_row(
                    "SELECT id FROM executions WHERE request_id=?1",
                    [&input.request_id],
                    |r| r.get(0),
                )
                .map_err(err)?;
            if crate::journal::original(c, id)? != submitted {
                return Err(
                    "This request identifier was already used with different journal content."
                        .into(),
                );
            }
            return Ok(());
        }
        return Err("This request identifier was already used for a different trade.".into());
    }
    let content = input.journal.clone().unwrap_or_default();
    content.validate(input.side)?;
    let cash: i64 = c
        .query_row(
            "SELECT COALESCE(SUM(amount_micros),0) FROM cash_ledger WHERE portfolio_id=1",
            [],
            |r| r.get(0),
        )
        .map_err(err)?;
    let history = executions(c)?;
    let states = states(&history)?;
    let position = states.get(&input.security_id).cloned().unwrap_or_default();
    let effect = engine::apply(
        &position,
        input.side,
        input.quantity,
        input.price_micros,
        input.brokerage_micros,
    )?;
    let new_cash = engine::bounded(i128::from(cash) + i128::from(effect.cash_delta))?;
    if new_cash < 0 {
        return Err("Insufficient available cash, including brokerage.".into());
    }
    if !c
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM securities WHERE id=?1)",
            [input.security_id],
            |r| r.get::<_, bool>(0),
        )
        .map_err(err)?
    {
        return Err("Security not found.".into());
    }
    c.execute("INSERT INTO executions(request_id,portfolio_id,security_id,side,quantity,price_micros,brokerage_micros,notional_micros) VALUES(?1,1,?2,?3,?4,?5,?6,?7)",params![input.request_id,input.security_id,input.side.as_str(),input.quantity,input.price_micros,input.brokerage_micros,effect.notional]).map_err(err)?;
    let execution_id = c.last_insert_rowid();
    c.execute("INSERT INTO cash_ledger(portfolio_id,kind,amount_micros,execution_id,description) VALUES(1,?1,?2,?3,?4)",params![input.side.as_str(),effect.cash_delta,execution_id,format!("Manual simulated {}",input.side.as_str())]).map_err(err)?;
    crate::journal::create(c, execution_id, input.side, &content)?;
    Ok(())
}
