// The chips of the Devil's Table: the CR-Tokens themselves, with a number on each. This is only the
// arithmetic of them: how an amount is stacked up from the seven kinds (see `ui/poker_chips.rs` for
// how they are drawn).
use crate::poker_table::Chips;

/// The kinds of chip, the biggest first.
pub const DENOMS: [Chips; 7] = [1000, 500, 100, 50, 25, 5, 1];

/// Most chips of one kind in a first pass (a stack of this many is already a tall one); the small
/// chips may stand a little taller, so that a bet of 40 is eight fives and not a heap of ones.
const TALL: Chips = 6;

fn cap(d: Chips) -> Chips {
    if d <= 5 {
        TALL * 2
    } else {
        TALL
    }
}

/// A kind is used when there would be three of it (two, for the small ones); the ones always are.
fn used(d: Chips, amount: Chips) -> bool {
    d == 1 || d * 3 <= amount || (d <= 5 && d * 2 <= amount)
}

/// How `amount` is stacked: (the number on the chip, how many), biggest first, nothing for 0. The
/// biggest kind used is one of which there would be at least three, so that a pile of chips looks like
/// one (a thousand is six hundreds, six fifties and four twenty-fives, not one big chip). The chips
/// always add up to exactly the amount.
pub fn breakdown(amount: Chips) -> Vec<(Chips, Chips)> {
    let mut counts = [0 as Chips; DENOMS.len()];
    let mut rest = amount;
    // first pass: the kinds that fit three times, up to a tall stack of each
    for (i, &d) in DENOMS.iter().enumerate() {
        if !used(d, amount) {
            continue;
        }
        let n = (rest / d).min(cap(d));
        counts[i] = n;
        rest -= n * d;
    }
    // what is left (a big amount) goes on the biggest kinds that are in use, then on to the smaller
    for (i, &d) in DENOMS.iter().enumerate() {
        if rest == 0 {
            break;
        }
        if !used(d, amount) {
            continue;
        }
        let n = rest / d;
        counts[i] += n;
        rest -= n * d;
    }
    DENOMS.iter().zip(counts).filter(|(_, n)| *n > 0).map(|(&d, n)| (d, n)).collect()
}

/// The number written on a chip: 1000 is "1K".
pub fn label(denom: Chips) -> String {
    if denom >= 1000 {
        format!("{}K", denom / 1000)
    } else {
        denom.to_string()
    }
}

/// The colour of a chip's rim and number: bone, blood, emerald, soul blue, gold, violet, ember.
pub fn colour(denom: Chips) -> [u8; 3] {
    match denom {
        1 => [226, 214, 196],
        5 => [214, 44, 48],
        25 => [52, 168, 104],
        50 => [112, 176, 240],
        100 => [222, 182, 88],
        500 => [160, 100, 226],
        _ => [255, 128, 48],
    }
}

/// How many chips there are in a breakdown.
pub fn count(stack: &[(Chips, Chips)]) -> Chips {
    stack.iter().map(|(_, n)| n).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn total(v: &[(Chips, Chips)]) -> Chips {
        v.iter().map(|(d, n)| d * n).sum()
    }

    #[test]
    fn the_chips_always_add_up_to_the_amount() {
        for a in (0..3000).chain([4999, 5000, 5001, 7777, 12345, 99999]) {
            let v = breakdown(a);
            assert_eq!(total(&v), a, "{a}: {v:?}");
        }
    }

    #[test]
    fn only_the_seven_kinds_are_used_and_biggest_first() {
        for a in [1, 7, 20, 40, 130, 500, 1000, 2600, 5000] {
            let v = breakdown(a);
            assert!(v.iter().all(|(d, n)| DENOMS.contains(d) && *n > 0), "{v:?}");
            assert!(v.windows(2).all(|w| w[0].0 > w[1].0), "{v:?}");
        }
        assert!(breakdown(0).is_empty());
    }

    #[test]
    fn a_thousand_is_a_pile_not_one_chip() {
        assert_eq!(breakdown(1000), vec![(100, 6), (50, 6), (25, 4)]);
    }

    #[test]
    fn small_bets_are_small_chips() {
        assert_eq!(breakdown(20), vec![(5, 4)]);
        assert_eq!(breakdown(10), vec![(5, 2)]);
        assert_eq!(breakdown(3), vec![(1, 3)]);
        assert_eq!(breakdown(500), vec![(100, 5)]);
        assert_eq!(breakdown(40), vec![(5, 8)]);
        assert_eq!(breakdown(130), vec![(25, 5), (5, 1)]);
    }

    #[test]
    fn the_whole_table_in_one_stack_is_five_thousand_chips_of_a_thousand() {
        assert_eq!(breakdown(5000), vec![(1000, 5)]);
    }

    #[test]
    fn no_tower_is_absurdly_tall() {
        for a in [1, 9, 40, 999, 1000, 1234, 3333, 5000] {
            for (d, n) in breakdown(a) {
                assert!(n <= 12 || d == 1 || d == 5, "{a}: {n} of {d}");
            }
        }
    }

    #[test]
    fn each_kind_has_its_own_label_and_colour() {
        let labels: Vec<String> = DENOMS.iter().map(|&d| label(d)).collect();
        assert_eq!(labels, ["1K", "500", "100", "50", "25", "5", "1"]);
        for (i, a) in DENOMS.iter().enumerate() {
            for b in &DENOMS[i + 1..] {
                assert_ne!(colour(*a), colour(*b));
            }
        }
    }
}
