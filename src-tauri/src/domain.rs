//! Exact fixed-point monetary values, independent of SQLite and the UI.
pub const MICROS_PER_AUD: i64 = 1_000_000;
// Keep IPC integers exactly representable in JavaScript and leave arithmetic headroom.
pub const MAX_MONEY: i64 = 1_000_000_000 * MICROS_PER_AUD;

pub fn validate_setup(name: &str, capital: i64, brokerage: i64) -> Result<(), String> {
    if name.trim().is_empty() || name.trim().chars().count() > 80 {
        return Err("Enter a trader name of 1–80 characters.".into());
    }
    if !(1..=MAX_MONEY).contains(&capital) || capital % 10_000 != 0 {
        return Err(
            "Starting capital must be positive, at most A$1 billion, and in whole cents.".into(),
        );
    }
    if !(0..=MAX_MONEY).contains(&brokerage) || brokerage % 10_000 != 0 {
        return Err(
            "Brokerage must be non-negative, at most A$1 billion, and in whole cents.".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ben_reference_is_exact_at_sub_cent_prices() {
        // Representation acceptance test; BUY execution belongs to Phase 2.
        let entry_price = 10_115_000_i64;
        let outlay = entry_price.checked_mul(24).unwrap() + 3 * MICROS_PER_AUD;
        assert_eq!(outlay, 245_760_000);
        assert_eq!(1000 * MICROS_PER_AUD - outlay, 754_240_000);
    }
    #[test]
    fn validates_money_and_profile_boundaries() {
        assert!(validate_setup("Trader", 1_000_000_000, 3_000_000).is_ok());
        for capital in [0, -1, 1, MAX_MONEY + 10_000] {
            assert!(validate_setup("Trader", capital, 0).is_err());
        }
        assert!(validate_setup(" ", 10_000, 0).is_err());
        assert!(validate_setup(&"a".repeat(81), 10_000, 0).is_err());
        assert!(validate_setup("Trader", 10_000, -10_000).is_err());
    }
}
