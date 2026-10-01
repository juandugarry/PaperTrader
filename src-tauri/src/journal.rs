//! Versioned commentary and derived position lifecycles. Fills remain immutable.
use crate::{
    domain::MAX_MONEY,
    engine::{self, PositionState, Side},
    trading::{self, Execution},
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Content {
    pub thesis: String,
    pub entry_trigger: String,
    pub target_micros: Option<i64>,
    pub stop_micros: Option<i64>,
    pub planned_risk_micros: Option<i64>,
    pub notes: String,
    pub exit_reason: String,
    pub followed_plan: Option<bool>,
    pub went_well: String,
    pub went_poorly: String,
    pub would_change: String,
}
impl Content {
    pub fn validate(&self, side: Side) -> Result<(), String> {
        for text in [
            &self.thesis,
            &self.entry_trigger,
            &self.notes,
            &self.exit_reason,
            &self.went_well,
            &self.went_poorly,
            &self.would_change,
        ] {
            if text.chars().count() > 4000 {
                return Err("Each journal text field must be at most 4,000 characters.".into());
            }
        }
        for price in [self.target_micros, self.stop_micros].into_iter().flatten() {
            if !(1..=MAX_MONEY).contains(&price) {
                return Err("Target and invalidation prices must be positive and within the supported limit.".into());
            }
        }
        if self
            .planned_risk_micros
            .is_some_and(|r| !(0..=MAX_MONEY).contains(&r) || r % 10_000 != 0)
        {
            return Err("Planned risk must be non-negative and in whole cents.".into());
        }
        match side {
            Side::BUY
                if !self.exit_reason.is_empty()
                    || self.followed_plan.is_some()
                    || !self.went_well.is_empty()
                    || !self.went_poorly.is_empty()
                    || !self.would_change.is_empty() =>
            {
                return Err("Exit reviews belong to SELL fills.".into())
            }
            Side::SELL
                if !self.thesis.is_empty()
                    || !self.entry_trigger.is_empty()
                    || self.target_micros.is_some()
                    || self.stop_micros.is_some()
                    || self.planned_risk_micros.is_some() =>
            {
                return Err("Entry plans belong to BUY fills.".into())
            }
            _ => {}
        }
        Ok(())
    }
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveJournal {
    pub execution_id: i64,
    pub expected_version: i64,
    pub content: Content,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Revision {
    pub version: i64,
    pub content: Content,
    pub created_at: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalFill {
    pub execution: Execution,
    pub captured_with_fill: bool,
    pub revisions: Vec<Revision>,
    pub gross_pnl_micros: Option<i64>,
    pub net_pnl_micros: Option<i64>,
    pub transaction_costs_micros: Option<i64>,
    pub released_cost_micros: Option<i64>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalTrade {
    pub id: i64,
    pub security_id: i64,
    pub ticker: String,
    pub name: String,
    pub opened_at: String,
    pub closed_at: Option<String>,
    pub quantity_bought: i64,
    pub quantity_sold: i64,
    pub quantity_open: i64,
    pub entry_cost_micros: i64,
    pub brokerage_paid_micros: i64,
    pub gross_pnl_micros: i64,
    pub net_pnl_micros: i64,
    pub realised_costs_micros: i64,
    pub realised_basis_micros: i64,
    pub fills: Vec<JournalFill>,
}
fn db_error(e: rusqlite::Error) -> String {
    e.to_string()
}
fn insert_revision(c: &Connection, id: i64, version: i64, content: &Content) -> Result<(), String> {
    c.execute("INSERT INTO journal_revisions(execution_id,version,thesis,entry_trigger,target_micros,stop_micros,planned_risk_micros,notes,exit_reason,followed_plan,went_well,went_poorly,would_change) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",params![id,version,content.thesis,content.entry_trigger,content.target_micros,content.stop_micros,content.planned_risk_micros,content.notes,content.exit_reason,content.followed_plan,content.went_well,content.went_poorly,content.would_change]).map_err(db_error)?;
    Ok(())
}
pub fn create(c: &Connection, id: i64, side: Side, content: &Content) -> Result<(), String> {
    content.validate(side)?;
    c.execute(
        "INSERT INTO journal_entries(execution_id,captured_with_fill) VALUES(?1,1)",
        [id],
    )
    .map_err(db_error)?;
    insert_revision(c, id, 1, content)
}
pub fn revisions(c: &Connection, id: i64) -> Result<Vec<Revision>, String> {
    let mut q=c.prepare("SELECT version,thesis,entry_trigger,target_micros,stop_micros,planned_risk_micros,notes,exit_reason,followed_plan,went_well,went_poorly,would_change,created_at FROM journal_revisions WHERE execution_id=?1 ORDER BY version").map_err(db_error)?;
    let result = q
        .query_map([id], |r| {
            Ok(Revision {
                version: r.get(0)?,
                content: Content {
                    thesis: r.get(1)?,
                    entry_trigger: r.get(2)?,
                    target_micros: r.get(3)?,
                    stop_micros: r.get(4)?,
                    planned_risk_micros: r.get(5)?,
                    notes: r.get(6)?,
                    exit_reason: r.get(7)?,
                    followed_plan: r.get(8)?,
                    went_well: r.get(9)?,
                    went_poorly: r.get(10)?,
                    would_change: r.get(11)?,
                },
                created_at: r.get(12)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error);
    result
}
pub fn original(c: &Connection, id: i64) -> Result<Content, String> {
    revisions(c, id)?
        .into_iter()
        .next()
        .map(|r| r.content)
        .ok_or_else(|| "Journal entry not found.".into())
}
pub fn save(c: &Connection, input: SaveJournal) -> Result<(), String> {
    let side = c
        .query_row(
            "SELECT side FROM executions WHERE id=?1",
            [input.execution_id],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(db_error)?
        .ok_or("Execution not found.")?;
    input
        .content
        .validate(if side == "BUY" { Side::BUY } else { Side::SELL })?;
    let history = revisions(c, input.execution_id)?;
    let latest = history.last().ok_or("Journal entry not found.")?;
    if latest.version != input.expected_version {
        return Err(
            "This journal entry changed in another window. Reload it before saving.".into(),
        );
    }
    if latest.content == input.content {
        return Ok(());
    }
    insert_revision(c, input.execution_id, latest.version + 1, &input.content)
}
pub fn read(c: &Connection) -> Result<Vec<JournalTrade>, String> {
    let history = trading::executions(c)?;
    let mut trades = Vec::<JournalTrade>::new();
    let mut active = BTreeMap::<i64, (usize, PositionState, PositionState)>::new();
    for execution in history {
        if execution.side == Side::BUY && !active.contains_key(&execution.security_id) {
            trades.push(JournalTrade {
                id: execution.id,
                security_id: execution.security_id,
                ticker: execution.ticker.clone(),
                name: execution.name.clone(),
                opened_at: execution.created_at.clone(),
                closed_at: None,
                quantity_bought: 0,
                quantity_sold: 0,
                quantity_open: 0,
                entry_cost_micros: 0,
                brokerage_paid_micros: 0,
                gross_pnl_micros: 0,
                net_pnl_micros: 0,
                realised_costs_micros: 0,
                realised_basis_micros: 0,
                fills: Vec::new(),
            });
            active.insert(
                execution.security_id,
                (
                    trades.len() - 1,
                    PositionState::default(),
                    PositionState::default(),
                ),
            );
        }
        let (index, state, without_fees) = active
            .get_mut(&execution.security_id)
            .ok_or("Sell without a linked journal lifecycle.")?;
        let effect = engine::apply(
            state,
            execution.side,
            execution.quantity,
            execution.price_micros,
            execution.brokerage_micros,
        )?;
        let gross_effect = engine::apply(
            without_fees,
            execution.side,
            execution.quantity,
            execution.price_micros,
            0,
        )?;
        let sell = execution.side == Side::SELL;
        let net_pnl = sell.then_some(effect.state.realised - state.realised);
        let gross_pnl = sell.then_some(gross_effect.state.realised - without_fees.realised);
        let released = sell.then_some(state.cost_basis - effect.state.cost_basis);
        let costs = gross_pnl.zip(net_pnl).map(|(g, n)| g - n);
        let revisions = revisions(c, execution.id)?;
        if revisions.is_empty() {
            return Err("Fill has no linked journal revision.".into());
        }
        let captured_with_fill = c
            .query_row(
                "SELECT captured_with_fill FROM journal_entries WHERE execution_id=?1",
                [execution.id],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        let trade = &mut trades[*index];
        if sell {
            trade.quantity_sold += execution.quantity;
            trade.realised_basis_micros = engine::bounded(
                i128::from(trade.realised_basis_micros) + i128::from(released.unwrap()),
            )?;
        } else {
            trade.quantity_bought += execution.quantity;
            trade.entry_cost_micros = engine::bounded(
                i128::from(trade.entry_cost_micros) - i128::from(execution.cash_delta_micros),
            )?;
        }
        trade.quantity_open = effect.state.quantity;
        trade.brokerage_paid_micros = effect.state.brokerage_paid;
        trade.gross_pnl_micros = gross_effect.state.realised;
        trade.net_pnl_micros = effect.state.realised;
        trade.realised_costs_micros = trade.gross_pnl_micros - trade.net_pnl_micros;
        trade.fills.push(JournalFill {
            execution,
            captured_with_fill,
            revisions,
            gross_pnl_micros: gross_pnl,
            net_pnl_micros: net_pnl,
            transaction_costs_micros: costs,
            released_cost_micros: released,
        });
        *state = effect.state;
        *without_fees = gross_effect.state;
        if state.quantity == 0 {
            trade.closed_at = Some(trade.fills.last().unwrap().execution.created_at.clone());
            active.remove(&trade.security_id);
        }
    }
    Ok(trades)
}
