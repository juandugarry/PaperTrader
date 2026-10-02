//! Fractional, virtual-only crypto accounting. AUD cash is exact integer micro-dollars.
use crate::{
    engine::{bounded, Side},
    funding,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
const MAX_ATOMS: i64 = 9_000_000_000_000_000_000;
const MAX_PRICE: i64 = 9_000_000_000_000_000_000;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub pair: String,
    pub symbol: String,
    pub name: String,
    pub ws_symbol: String,
    pub quote: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Setup {
    pub mode: String,
    pub starting_funds_micros: i64,
    pub request_id: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trade {
    pub request_id: String,
    pub pair: String,
    pub side: Side,
    pub quantity_atoms: String,
    pub price_picos: String,
    pub fee_micros: i64,
    pub notes: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Wallet {
    pub mode: String,
    pub cash_micros: i64,
    pub contributions_micros: i64,
    pub portfolio_value_micros: Option<i64>,
    pub total_return_micros: Option<i64>,
    pub realised_pnl_micros: i64,
    pub fee_micros: i64,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub pair: String,
    pub symbol: String,
    pub quantity_atoms: String,
    pub cost_basis_micros: i64,
    pub price_picos: Option<String>,
    pub fetched_at: Option<String>,
    pub value_micros: Option<i64>,
    pub unrealised_pnl_micros: Option<i64>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Execution {
    pub id: i64,
    pub pair: String,
    pub symbol: String,
    pub side: String,
    pub quantity_atoms: String,
    pub price_picos: String,
    pub fee_micros: i64,
    pub notional_micros: i64,
    pub cash_delta_micros: i64,
    pub notes: String,
    pub created_at: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub wallet: Option<Wallet>,
    pub positions: Vec<Position>,
    pub executions: Vec<Execution>,
    pub asset_value_micros: Option<i64>,
}
#[derive(Default)]
struct State {
    atoms: i64,
    basis: i64,
    realised: i64,
    fees: i64,
}
pub fn amount(atoms: i64, price: i64) -> Result<i64, String> {
    if !(1..=MAX_ATOMS).contains(&atoms) || !(1..=MAX_PRICE).contains(&price) {
        return Err(
            "Enter a positive crypto amount (up to 8 decimals) and price within supported limits."
                .into(),
        );
    }
    bounded(
        ((i128::from(atoms) * i128::from(price) + 500_000_000_000_000_000)
            / 1_000_000_000_000_000_000)
            * 10_000,
    )
}
fn apply(
    state: &mut State,
    side: Side,
    atoms: i64,
    price: i64,
    fee: i64,
) -> Result<(i64, i64), String> {
    let notional = amount(atoms, price)?;
    if notional < 10_000 || !(0..=crate::domain::MAX_MONEY).contains(&fee) || fee % 10_000 != 0 {
        return Err(
            "The trade must round to at least one cent; fees must be in whole cents.".into(),
        );
    }
    let delta = match side {
        Side::BUY => {
            state.atoms = state
                .atoms
                .checked_add(atoms)
                .filter(|q| *q <= MAX_ATOMS)
                .ok_or("Crypto quantity limit exceeded")?;
            let outlay = bounded(i128::from(notional) + i128::from(fee))?;
            state.basis = bounded(i128::from(state.basis) + i128::from(outlay))?;
            -outlay
        }
        Side::SELL => {
            if atoms > state.atoms {
                return Err("You cannot sell more crypto than you own.".into());
            }
            let released = if atoms == state.atoms {
                state.basis
            } else {
                ((i128::from(state.basis) * i128::from(atoms) + i128::from(state.atoms) / 2)
                    / i128::from(state.atoms)) as i64
            };
            let proceeds = bounded(i128::from(notional) - i128::from(fee))?;
            state.realised =
                bounded(i128::from(state.realised) + i128::from(proceeds) - i128::from(released))?;
            state.basis -= released;
            state.atoms -= atoms;
            proceeds
        }
    };
    state.fees = bounded(i128::from(state.fees) + i128::from(fee))?;
    Ok((notional, delta))
}
pub fn mode(c: &Connection) -> Result<Option<String>, String> {
    c.query_row("SELECT mode FROM crypto_wallet WHERE id=1", [], |r| {
        r.get(0)
    })
    .optional()
    .map_err(|e| e.to_string())
}
pub fn setup(c: &Connection, input: Setup) -> Result<(), String> {
    funding::valid_request(&input.request_id)?;
    let prior: Option<(String, String, i64)> = c
        .query_row(
            "SELECT mode,request_id,starting_funds_micros FROM crypto_wallet WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some(prior) = prior {
        return if prior == (input.mode, input.request_id, input.starting_funds_micros) {
            Ok(())
        } else {
            Err("Your crypto wallet is already set up.".into())
        };
    }
    if !["separate", "shared"].contains(&input.mode.as_str())
        || (input.mode == "shared" && input.starting_funds_micros != 0)
    {
        return Err("Choose separate or shared virtual funds.".into());
    }
    c.execute(
        "INSERT INTO crypto_wallet(id,mode,request_id,starting_funds_micros) VALUES(1,?1,?2,?3)",
        params![input.mode, input.request_id, input.starting_funds_micros],
    )
    .map_err(|e| e.to_string())?;
    if input.mode == "separate" {
        funding::deposit(
            c,
            funding::Deposit {
                request_id: input.request_id,
                account: "crypto".into(),
                amount_micros: input.starting_funds_micros,
                description: "Initial virtual crypto funds".into(),
            },
        )?;
    }
    Ok(())
}
pub fn catalog(c: &Connection) -> Result<Vec<Asset>, String> {
    let body: Option<String> = c
        .query_row("SELECT body FROM crypto_catalog WHERE id=1", [], |r| {
            r.get(0)
        })
        .optional()
        .map_err(|e| e.to_string())?;
    body.map(|body| {
        serde_json::from_str(&body).map_err(|_| "Invalid cached crypto directory.".into())
    })
    .unwrap_or(Ok(vec![]))
}
fn history(c: &Connection) -> Result<Vec<Execution>, String> {
    let mut q=c.prepare("SELECT id,pair,symbol,side,quantity_atoms,price_picos,fee_micros,notional_micros,cash_delta_micros,notes,created_at FROM crypto_executions ORDER BY id").map_err(|e|e.to_string())?;
    let rows = q
        .query_map([], |r| {
            Ok(Execution {
                id: r.get(0)?,
                pair: r.get(1)?,
                symbol: r.get(2)?,
                side: r.get(3)?,
                quantity_atoms: r.get::<_, i64>(4)?.to_string(),
                price_picos: r.get::<_, i64>(5)?.to_string(),
                fee_micros: r.get(6)?,
                notional_micros: r.get(7)?,
                cash_delta_micros: r.get(8)?,
                notes: r.get(9)?,
                created_at: r.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string());
    rows
}
pub fn read(c: &Connection) -> Result<Snapshot, String> {
    let executions = history(c)?;
    let mut states = BTreeMap::<String, (String, State)>::new();
    for e in &executions {
        let (_, state) = states
            .entry(e.pair.clone())
            .or_insert_with(|| (e.symbol.clone(), State::default()));
        let effect = apply(
            state,
            if e.side == "BUY" {
                Side::BUY
            } else {
                Side::SELL
            },
            e.quantity_atoms
                .parse()
                .map_err(|_| "Invalid crypto quantity")?,
            e.price_picos.parse().map_err(|_| "Invalid crypto price")?,
            e.fee_micros,
        )?;
        if effect != (e.notional_micros, e.cash_delta_micros) {
            return Err("Crypto fill and cash movement disagree.".into());
        }
    }
    let mut positions = vec![];
    let mut value = Some(0_i128);
    let mut realised = 0_i128;
    let mut fees = 0_i128;
    for (pair, (symbol, state)) in states {
        realised += i128::from(state.realised);
        fees += i128::from(state.fees);
        if state.atoms == 0 {
            continue;
        }
        let quote: Option<(i64, String)> = c
            .query_row(
                "SELECT price_picos,fetched_at FROM crypto_quotes WHERE pair=?1",
                [&pair],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let mark = quote
            .as_ref()
            .map(|(price, _)| amount(state.atoms, *price))
            .transpose()?;
        value = value.zip(mark).map(|(v, m)| v + i128::from(m));
        positions.push(Position {
            pair,
            symbol,
            quantity_atoms: state.atoms.to_string(),
            cost_basis_micros: state.basis,
            price_picos: quote.as_ref().map(|(price, _)| price.to_string()),
            fetched_at: quote.map(|(_, t)| t),
            value_micros: mark,
            unrealised_pnl_micros: mark.map(|m| m - state.basis),
        });
    }
    let asset_value = value.map(bounded).transpose()?;
    let wallet = if let Some(mode) = mode(c)? {
        let (cash, contributions) = if mode == "shared" {
            (funding::cash(c)?, funding::stock_contributions(c)?)
        } else {
            let deposits:i64=c.query_row("SELECT COALESCE(SUM(amount_micros),0) FROM virtual_deposits WHERE account='crypto'",[],|r|r.get(0)).map_err(|e|e.to_string())?;
            let moves: i64 = c
                .query_row(
                    "SELECT COALESCE(SUM(cash_delta_micros),0) FROM crypto_executions",
                    [],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            (
                bounded(i128::from(deposits) + i128::from(moves))?,
                bounded(i128::from(deposits))?,
            )
        };
        let base = if mode == "shared" {
            crate::trading::read(c, cash, contributions)?.portfolio_value_micros
        } else {
            Some(cash)
        };
        let portfolio = asset_value
            .zip(base)
            .map(|(v, b)| bounded(i128::from(v) + i128::from(b)))
            .transpose()?;
        Some(Wallet {
            mode,
            cash_micros: cash,
            contributions_micros: contributions,
            portfolio_value_micros: portfolio,
            total_return_micros: portfolio.map(|v| v - contributions),
            realised_pnl_micros: bounded(realised)?,
            fee_micros: bounded(fees)?,
        })
    } else {
        None
    };
    Ok(Snapshot {
        wallet,
        positions,
        executions,
        asset_value_micros: asset_value,
    })
}
pub fn execute(c: &Connection, input: Trade) -> Result<(), String> {
    funding::valid_request(&input.request_id)?;
    if input.notes.chars().count() > 10000 {
        return Err("Crypto trade notes may contain up to 10,000 characters.".into());
    }
    let atoms: i64 = input
        .quantity_atoms
        .parse()
        .map_err(|_| "Invalid crypto quantity")?;
    let price: i64 = input
        .price_picos
        .parse()
        .map_err(|_| "Invalid crypto price")?;
    let snapshot = read(c)?;
    let wallet = snapshot.wallet.ok_or("Set up your crypto wallet first.")?;
    if let Some(prior)=c.query_row("SELECT pair,side,quantity_atoms,price_picos,fee_micros,notes FROM crypto_executions WHERE request_id=?1",[&input.request_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,r.get::<_,i64>(3)?,r.get::<_,i64>(4)?,r.get::<_,String>(5)?))).optional().map_err(|e|e.to_string())?{return if prior==(input.pair,input.side.as_str().to_string(),atoms,price,input.fee_micros,input.notes){Ok(())}else{Err("Crypto request ID reused with different details.".into())}}
    let asset = catalog(c)?
        .into_iter()
        .find(|a| a.pair == input.pair)
        .ok_or("Choose a crypto asset from the cached directory.")?;
    let mut state = State::default();
    for e in snapshot.executions.iter().filter(|e| e.pair == input.pair) {
        apply(
            &mut state,
            if e.side == "BUY" {
                Side::BUY
            } else {
                Side::SELL
            },
            e.quantity_atoms.parse().map_err(|_| "Invalid quantity")?,
            e.price_picos.parse().map_err(|_| "Invalid price")?,
            e.fee_micros,
        )?;
    }
    let (notional, delta) = apply(&mut state, input.side, atoms, price, input.fee_micros)?;
    let remaining = bounded(i128::from(wallet.cash_micros) + i128::from(delta))?;
    if remaining < 0 {
        return Err("Insufficient virtual funds for this crypto trade and fee.".into());
    }
    c.execute("INSERT INTO crypto_executions(request_id,pair,symbol,side,quantity_atoms,price_picos,fee_micros,notional_micros,cash_delta_micros,notes) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![input.request_id,input.pair,asset.symbol,input.side.as_str(),atoms,price,input.fee_micros,notional,delta,input.notes]).map_err(|e|e.to_string())?;
    Ok(())
}
pub fn stock_snapshot(c: &Connection) -> Result<crate::trading::TradingSnapshot, String> {
    let contributions = funding::stock_contributions(c)?;
    let mut stocks = crate::trading::read(c, funding::cash(c)?, contributions)?;
    let crypto = read(c)?;
    if crypto.wallet.as_ref().is_some_and(|w| w.mode == "shared") {
        stocks.portfolio_value_micros = stocks
            .portfolio_value_micros
            .zip(crypto.asset_value_micros)
            .map(|(s, c)| bounded(i128::from(s) + i128::from(c)))
            .transpose()?;
        stocks.total_return_micros = stocks.portfolio_value_micros.map(|v| v - contributions);
    }
    Ok(stocks)
}
