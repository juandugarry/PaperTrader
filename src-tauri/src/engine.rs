//! Pure average-cost accounting; no database, network or UI dependencies.
use crate::domain::MAX_MONEY;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum Side {
    BUY,
    SELL,
}
impl Side {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BUY => "BUY",
            Self::SELL => "SELL",
        }
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PositionState {
    pub quantity: i64,
    pub gross_basis: i64,
    pub cost_basis: i64,
    pub realised: i64,
    pub brokerage_paid: i64,
}
#[derive(Debug)]
pub struct Effect {
    pub state: PositionState,
    pub notional: i64,
    pub cash_delta: i64,
}
pub fn bounded(value: i128) -> Result<i64, String> {
    if value.abs() > i128::from(MAX_MONEY) {
        return Err("Amount exceeds the supported A$1 billion accounting limit.".into());
    }
    Ok(value as i64)
}
pub fn market_value(quantity: i64, price: i64) -> Result<i64, String> {
    if !(1..=1_000_000_000).contains(&quantity) {
        return Err("Quantity must be 1–1,000,000,000 whole shares.".into());
    }
    if !(1..=MAX_MONEY).contains(&price) {
        return Err("Price must be positive and within the supported limit.".into());
    }
    // Round positive execution totals to nearest cent, half up, using wide integer arithmetic.
    let amount = bounded(((i128::from(quantity) * i128::from(price) + 5_000) / 10_000) * 10_000)?;
    Ok(amount)
}
pub fn notional(quantity: i64, price: i64) -> Result<i64, String> {
    let amount = market_value(quantity, price)?;
    if amount == 0 {
        return Err("The total share value must round to at least one cent.".into());
    }
    Ok(amount)
}
fn allocate(basis: i64, sold: i64, owned: i64) -> i64 {
    if sold == owned {
        basis
    } else {
        ((i128::from(basis) * i128::from(sold) + i128::from(owned) / 2) / i128::from(owned)) as i64
    }
}
pub fn apply(
    position: &PositionState,
    side: Side,
    quantity: i64,
    price: i64,
    brokerage: i64,
) -> Result<Effect, String> {
    if !(0..=MAX_MONEY).contains(&brokerage) || brokerage % 10_000 != 0 {
        return Err("Brokerage must be non-negative and in whole cents.".into());
    }
    let amount = notional(quantity, price)?;
    let mut state = position.clone();
    state.brokerage_paid = bounded(i128::from(state.brokerage_paid) + i128::from(brokerage))?;
    let delta = match side {
        Side::BUY => {
            state.quantity = state
                .quantity
                .checked_add(quantity)
                .ok_or("Quantity overflow")?;
            if state.quantity > 1_000_000_000 {
                return Err("Position exceeds the whole-share quantity limit.".into());
            }
            state.gross_basis =
                bounded(i128::from(state.gross_basis) + i128::from(quantity) * i128::from(price))?;
            let outlay = bounded(i128::from(amount) + i128::from(brokerage))?;
            state.cost_basis = bounded(i128::from(state.cost_basis) + i128::from(outlay))?;
            -outlay
        }
        Side::SELL => {
            if quantity > state.quantity {
                return Err("You cannot sell more shares than you own.".into());
            }
            let released = allocate(state.cost_basis, quantity, state.quantity);
            let gross_released = allocate(state.gross_basis, quantity, state.quantity);
            let proceeds = bounded(i128::from(amount) - i128::from(brokerage))?;
            state.realised =
                bounded(i128::from(state.realised) + i128::from(proceeds) - i128::from(released))?;
            state.cost_basis -= released;
            state.gross_basis -= gross_released;
            state.quantity -= quantity;
            proceeds
        }
    };
    Ok(Effect {
        state,
        notional: amount,
        cash_delta: delta,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn averages_buys_and_allocates_brokerage_on_partial_and_full_sale() {
        let s = apply(
            &PositionState::default(),
            Side::BUY,
            10,
            10_000_000,
            3_000_000,
        )
        .unwrap()
        .state;
        let s = apply(&s, Side::BUY, 10, 12_000_000, 3_000_000)
            .unwrap()
            .state;
        assert_eq!(
            (s.quantity, s.gross_basis, s.cost_basis),
            (20, 220_000_000, 226_000_000)
        );
        let e = apply(&s, Side::SELL, 5, 15_000_000, 3_000_000).unwrap();
        assert_eq!(
            (e.cash_delta, e.state.cost_basis, e.state.realised),
            (72_000_000, 169_500_000, 15_500_000)
        );
        let e = apply(&e.state, Side::SELL, 15, 9_000_000, 3_000_000).unwrap();
        assert_eq!(
            (e.state.quantity, e.state.cost_basis, e.state.gross_basis),
            (0, 0, 0)
        );
        assert_eq!(e.state.realised, -22_000_000);
        assert_eq!(e.state.brokerage_paid, 12_000_000);
    }
    #[test]
    fn final_sale_consumes_all_rounding_remainder_and_rebuy_starts_new_basis() {
        let s = apply(&PositionState::default(), Side::BUY, 3, 333_333, 0)
            .unwrap()
            .state;
        assert_eq!(s.cost_basis, 1_000_000);
        let s = apply(&s, Side::SELL, 1, 333_333, 0).unwrap().state;
        assert_eq!(s.cost_basis, 666_667);
        let s = apply(&s, Side::SELL, 2, 333_333, 0).unwrap().state;
        assert_eq!(s.cost_basis, 0);
        assert_eq!(s.realised, 0);
        let s = apply(&s, Side::BUY, 1, 1_000_000, 0).unwrap().state;
        assert_eq!(s.cost_basis, 1_000_000);
    }
    #[test]
    fn rounds_half_cents_up_and_rejects_invalid_or_overflowing_orders() {
        assert_eq!(notional(1, 10_115_000).unwrap(), 10_120_000);
        for (q, p) in [
            (0, 1_000_000),
            (-1, 1_000_000),
            (1, 0),
            (1, 1),
            (1_000_000_000, MAX_MONEY),
        ] {
            assert!(notional(q, p).is_err());
        }
        assert!(apply(&PositionState::default(), Side::SELL, 1, 1_000_000, 0).is_err());
        assert!(apply(&PositionState::default(), Side::BUY, 1, 1_000_000, -1).is_err());
    }
    #[test]
    fn loss_and_brokerage_can_make_sale_cash_negative() {
        let s = apply(&PositionState::default(), Side::BUY, 1, 1_000_000, 0)
            .unwrap()
            .state;
        let e = apply(&s, Side::SELL, 1, 100_000, 3_000_000).unwrap();
        assert_eq!(e.cash_delta, -2_900_000);
        assert_eq!(e.state.realised, -3_900_000);
    }
    #[test]
    fn sub_cent_average_entry_is_preserved_and_partial_rounding_reconciles() {
        let e = apply(&PositionState::default(), Side::BUY, 1, 10_115_000, 0).unwrap();
        assert_eq!(e.state.gross_basis, 10_115_000);
        assert_eq!(e.state.cost_basis, 10_120_000);
        let e = apply(&PositionState::default(), Side::BUY, 7, 1_234_567, 30_000).unwrap();
        assert_eq!(e.cash_delta, -8_670_000);
        let e = apply(&e.state, Side::SELL, 3, 1_345_678, 20_000).unwrap();
        assert_eq!(e.state.cost_basis, 4_954_286);
        let e = apply(&e.state, Side::SELL, 1, 1_345_678, 20_000).unwrap();
        let e = apply(&e.state, Side::SELL, 3, 1_345_678, 20_000).unwrap();
        assert_eq!(e.state.cost_basis, 0);
        assert_eq!(e.state.gross_basis, 0);
        assert_eq!(e.state.realised, 700_000);
    }
}
