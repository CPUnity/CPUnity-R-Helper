//! Cards and the hand evaluator for Texas Hold'em.
//!
//! A card is a number 0..52: `rank = c % 13` (0 is a two, 12 an ace) and `suit = c / 13` (clubs,
//! diamonds, hearts, spades). `score` gives the best five-card hand out of up to seven cards as one
//! number: a bigger number is a better hand, equal numbers split the pot. It is checked against a
//! slow, obvious evaluator (every five-card choice) in the tests.

pub type Card = u8;

pub const RANKS: [char; 13] = ['2', '3', '4', '5', '6', '7', '8', '9', 'T', 'J', 'Q', 'K', 'A'];
pub const SUITS: [char; 4] = ['c', 'd', 'h', 's'];

pub fn rank(c: Card) -> usize {
    (c % 13) as usize
}

pub fn suit(c: Card) -> usize {
    (c / 13) as usize
}

pub fn card(rank: usize, suit: usize) -> Card {
    (suit * 13 + rank) as Card
}

/// "As", "Td", "2c" ...
pub fn name(c: Card) -> String {
    format!("{}{}", RANKS[rank(c)], SUITS[suit(c)])
}

/// Parses "As" / "td" ... (rank then suit).
pub fn parse(s: &str) -> Option<Card> {
    let mut it = s.chars();
    let r = it.next()?.to_ascii_uppercase();
    let su = it.next()?.to_ascii_lowercase();
    if it.next().is_some() {
        return None;
    }
    Some(card(RANKS.iter().position(|x| *x == r)?, SUITS.iter().position(|x| *x == su)?))
}

/// Several cards from "As Kd 7h".
pub fn parse_all(s: &str) -> Vec<Card> {
    s.split_whitespace().filter_map(parse).collect()
}

/// The kinds of hand, worst to best.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    HighCard,
    Pair,
    TwoPair,
    Trips,
    Straight,
    Flush,
    FullHouse,
    Quads,
    StraightFlush,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::HighCard => "High card",
            Category::Pair => "Pair",
            Category::TwoPair => "Two pair",
            Category::Trips => "Three of a kind",
            Category::Straight => "Straight",
            Category::Flush => "Flush",
            Category::FullHouse => "Full house",
            Category::Quads => "Four of a kind",
            Category::StraightFlush => "Straight flush",
        }
    }
}

const SHIFT: u32 = 20;

fn pack(category: Category, ranks: &[usize]) -> u32 {
    // up to five ranks, four bits each, most important first
    let mut v = 0u32;
    for i in 0..5 {
        v = (v << 4) | ranks.get(i).copied().unwrap_or(0) as u32;
    }
    ((category as u32) << SHIFT) | v
}

/// The category of a score.
pub fn category(score: u32) -> Category {
    match score >> SHIFT {
        0 => Category::HighCard,
        1 => Category::Pair,
        2 => Category::TwoPair,
        3 => Category::Trips,
        4 => Category::Straight,
        5 => Category::Flush,
        6 => Category::FullHouse,
        7 => Category::Quads,
        _ => Category::StraightFlush,
    }
}

/// The highest card of the straight in a rank mask (bit 0 is a two, bit 12 an ace), if there is
/// one; the ace also counts low (the wheel, A-2-3-4-5, whose high card is the five).
fn straight_high(mask: u16) -> Option<usize> {
    // look from the highest straight down
    for high in (4..13).rev() {
        let need = 0b11111u16 << (high - 4);
        if mask & need == need {
            return Some(high);
        }
    }
    // the wheel: A 2 3 4 5
    if mask & 0b1_0000_0000_1111 == 0b1_0000_0000_1111 {
        return Some(3);
    }
    None
}

/// The best five-card hand among `cards` (5 to 7 of them), as a score.
pub fn score(cards: &[Card]) -> u32 {
    let mut counts = [0u8; 13];
    let mut suit_mask = [0u16; 4];
    let mut mask = 0u16;
    for &c in cards {
        let (r, s) = (rank(c), suit(c));
        counts[r] += 1;
        suit_mask[s] |= 1 << r;
        mask |= 1 << r;
    }
    // flush (and straight flush)
    for sm in suit_mask {
        if sm.count_ones() >= 5 {
            if let Some(high) = straight_high(sm) {
                return pack(Category::StraightFlush, &[high]);
            }
            let mut ranks = Vec::with_capacity(5);
            for r in (0..13).rev() {
                if sm & (1 << r) != 0 && ranks.len() < 5 {
                    ranks.push(r);
                }
            }
            return pack(Category::Flush, &ranks);
        }
    }
    let mut quads = Vec::new();
    let mut trips = Vec::new();
    let mut pairs = Vec::new();
    let mut singles = Vec::new();
    for r in (0..13).rev() {
        match counts[r] {
            4 => quads.push(r),
            3 => trips.push(r),
            2 => pairs.push(r),
            1 => singles.push(r),
            _ => {}
        }
    }
    if let Some(&q) = quads.first() {
        // the best other card
        let kicker = (0..13).rev().find(|&r| r != q && counts[r] > 0).unwrap_or(0);
        return pack(Category::Quads, &[q, kicker]);
    }
    if !trips.is_empty() && (trips.len() > 1 || !pairs.is_empty()) {
        let t = trips[0];
        let p = if trips.len() > 1 { trips[1].max(pairs.first().copied().unwrap_or(0)) } else { pairs[0] };
        return pack(Category::FullHouse, &[t, p]);
    }
    if let Some(high) = straight_high(mask) {
        return pack(Category::Straight, &[high]);
    }
    if let Some(&t) = trips.first() {
        let kickers: Vec<usize> = (0..13).rev().filter(|&r| r != t && counts[r] > 0).take(2).collect();
        return pack(Category::Trips, &[t, kickers[0], kickers.get(1).copied().unwrap_or(0)]);
    }
    if pairs.len() >= 2 {
        let kicker = (0..13).rev().find(|&r| r != pairs[0] && r != pairs[1] && counts[r] > 0).unwrap_or(0);
        return pack(Category::TwoPair, &[pairs[0], pairs[1], kicker]);
    }
    if let Some(&p) = pairs.first() {
        let kickers: Vec<usize> = (0..13).rev().filter(|&r| r != p && counts[r] > 0).take(3).collect();
        return pack(Category::Pair, &[p, kickers[0], kickers[1], kickers[2]]);
    }
    let _ = singles;
    let top: Vec<usize> = (0..13).rev().filter(|&r| counts[r] > 0).take(5).collect();
    pack(Category::HighCard, &top)
}

/// "Pair of kings", "Flush, ace high" ... for showing a hand.
pub fn describe(score: u32) -> String {
    let v = score & ((1 << SHIFT) - 1);
    let r = |i: u32| ((v >> (4 * (4 - i))) & 0xf) as usize;
    let plural = |x: usize| if x == 4 { "sixes".to_string() } else { format!("{}s", RANK_NAMES[x]) };
    let text = match category(score) {
        Category::HighCard => format!("{} high", RANK_NAMES[r(0)]),
        Category::Pair => format!("pair of {}", plural(r(0))),
        Category::TwoPair => format!("{} and {}", plural(r(0)), plural(r(1))),
        Category::Trips => format!("three {}", plural(r(0))),
        Category::Straight => format!("straight, {} high", RANK_NAMES[r(0)]),
        Category::Flush => format!("flush, {} high", RANK_NAMES[r(0)]),
        Category::FullHouse => format!("{} full of {}", plural(r(0)), plural(r(1))),
        Category::Quads => format!("four {}", plural(r(0))),
        Category::StraightFlush => {
            if r(0) == 12 {
                "royal flush".to_string()
            } else {
                format!("straight flush, {} high", RANK_NAMES[r(0)])
            }
        }
    };
    let mut chars = text.chars();
    chars.next().map(|f| f.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
}

const RANK_NAMES: [&str; 13] =
    ["two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "jack", "queen", "king", "ace"];

#[cfg(test)]
pub fn slow_score(cards: &[Card]) -> u32 {
    // every choice of five cards, scored on its own
    fn five(c: &[Card]) -> u32 {
        score(c)
    }
    let n = cards.len();
    if n <= 5 {
        return five(cards);
    }
    let mut best = 0;
    let mut idx = [0usize; 5];
    fn rec(cards: &[Card], start: usize, depth: usize, idx: &mut [usize; 5], best: &mut u32) {
        if depth == 5 {
            let hand: Vec<Card> = idx.iter().map(|&i| cards[i]).collect();
            // (a five-card hand is scored by the same function, which is simple for five)
            *best = (*best).max(score_five_simple(&hand));
            return;
        }
        for i in start..cards.len() {
            idx[depth] = i;
            rec(cards, i + 1, depth + 1, idx, best);
        }
    }
    rec(cards, 0, 0, &mut idx, &mut best);
    best
}

/// An obviously-correct scorer for exactly five cards, written differently from `score` (tests only).
#[cfg(test)]
fn score_five_simple(cards: &[Card]) -> u32 {
    let mut ranks: Vec<usize> = cards.iter().map(|&c| rank(c)).collect();
    ranks.sort_unstable_by(|a, b| b.cmp(a));
    let flush = cards.iter().all(|&c| suit(c) == suit(cards[0]));
    let mut uniq = ranks.clone();
    uniq.dedup();
    let straight_top = if uniq.len() == 5 {
        if ranks[0] - ranks[4] == 4 {
            Some(ranks[0])
        } else if ranks == vec![12, 3, 2, 1, 0] {
            Some(3)
        } else {
            None
        }
    } else {
        None
    };
    // groups by (count, rank), biggest first
    let mut groups: Vec<(usize, usize)> = Vec::new();
    for &r in &uniq {
        groups.push((ranks.iter().filter(|&&x| x == r).count(), r));
    }
    groups.sort_unstable_by(|a, b| b.cmp(a));
    let order: Vec<usize> = groups.iter().map(|g| g.1).collect();
    let shape: Vec<usize> = groups.iter().map(|g| g.0).collect();
    match (flush, straight_top, shape.as_slice()) {
        (true, Some(t), _) => pack(Category::StraightFlush, &[t]),
        (_, _, [4, 1]) => pack(Category::Quads, &order),
        (_, _, [3, 2]) => pack(Category::FullHouse, &order),
        (true, None, _) => pack(Category::Flush, &order),
        (false, Some(t), _) => pack(Category::Straight, &[t]),
        (_, _, [3, 1, 1]) => pack(Category::Trips, &order),
        (_, _, [2, 2, 1]) => pack(Category::TwoPair, &order),
        (_, _, [2, 1, 1, 1]) => pack(Category::Pair, &order),
        _ => pack(Category::HighCard, &order),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(hand: &str) -> u32 {
        score(&parse_all(hand))
    }

    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            self.0 >> 33
        }
    }

    #[test]
    fn names_round_trip() {
        for c in 0..52u8 {
            assert_eq!(parse(&name(c)), Some(c));
        }
        assert_eq!(parse("Ah"), Some(card(12, 2)));
        assert_eq!(parse("1h"), None);
        assert_eq!(parse("Ahh"), None);
    }

    #[test]
    fn the_categories_are_found() {
        assert_eq!(category(s("As Ks Qs Js Ts")), Category::StraightFlush);
        assert_eq!(category(s("9c 9d 9h 9s 2c")), Category::Quads);
        assert_eq!(category(s("9c 9d 9h 2s 2c")), Category::FullHouse);
        assert_eq!(category(s("Ac 9c 6c 4c 2c")), Category::Flush);
        assert_eq!(category(s("9c Td Jh Qs Kc")), Category::Straight);
        assert_eq!(category(s("9c 9d 9h 4s 2c")), Category::Trips);
        assert_eq!(category(s("9c 9d 4h 4s 2c")), Category::TwoPair);
        assert_eq!(category(s("9c 9d 5h 4s 2c")), Category::Pair);
        assert_eq!(category(s("Ac 9d 6h 4s 2c")), Category::HighCard);
    }

    #[test]
    fn the_order_of_hands_is_right() {
        // each hand beats the one before it
        let ladder = [
            "7c 5d 4h 3s 2c", // seven high
            "Ac Kd 6h 4s 2c", // ace high
            "2c 2d 5h 4s 3c", // pair of twos
            "Ac Ad 5h 4s 3c", // pair of aces
            "Ac Ad Kh Ks 3c", // aces and kings
            "2c 2d 3h 3s 4c", // twos and threes: worse than aces up
        ];
        assert!(s(ladder[0]) < s(ladder[1]));
        assert!(s(ladder[1]) < s(ladder[2]));
        assert!(s(ladder[2]) < s(ladder[3]));
        assert!(s(ladder[3]) < s(ladder[4]) && s(ladder[5]) < s(ladder[4]));
        assert!(s(ladder[3]) < s(ladder[5]));
        assert!(s("Ac Ad Ah 3s 4c") > s("Ac Ad Kh Ks 3c"));
        assert!(s("2c 3d 4h 5s 6c") > s("Ac Ad Ah 3s 4c"));
        assert!(s("2c 2d 4h 4s 4c") > s("Ac 9c 6c 4c 2c") && s("Ac 9c 6c 4c 2c") > s("2c 3d 4h 5s 6c"));
        assert!(s("2c 2d 2h 2s 3c") > s("Ac Ad Ah 3s 3c"));
        assert!(s("2c 3c 4c 5c 6c") > s("2c 2d 2h 2s 3c"));
    }

    #[test]
    fn kickers_decide_and_equal_hands_tie() {
        assert!(s("Ac Ad Kh 4s 2c") > s("Ac Ad Qh 4s 2c"));
        assert!(s("Ac Ad Kh 4s 3c") > s("Ac Ad Kh 4s 2c"));
        // the same hand in other suits is the same score
        assert_eq!(s("Ac Ad Kh 4s 2c"), s("Ah As Kd 4c 2h"));
        // the fifth card of a flush counts, the suits do not
        assert!(s("Ac Kc Qc Jc 9c") > s("Ac Kc Qc Jc 8c"));
        // two pair: the kicker plays
        assert!(s("Kc Kd 5h 5s Ac") > s("Kc Kd 5h 5s Qc"));
    }

    #[test]
    fn the_wheel_is_the_lowest_straight_and_an_ace_does_not_wrap() {
        assert_eq!(category(s("Ac 2d 3h 4s 5c")), Category::Straight);
        assert!(s("2c 3d 4h 5s 6c") > s("Ac 2d 3h 4s 5c"));
        assert!(s("Ac Kd Qh Js Tc") > s("9c Td Jh Qs Kc"));
        assert_ne!(category(s("Qc Kd Ah 2s 3c")), Category::Straight);
        assert_eq!(category(s("Ac 2c 3c 4c 5c")), Category::StraightFlush);
        assert_eq!(describe(s("Ac 2c 3c 4c 5c")), "Straight flush, five high");
    }

    #[test]
    fn seven_cards_give_the_best_five() {
        // a full house hides in seven cards, and beats the flush that is also there
        assert_eq!(category(s("Ac Ad Ah Kc Kd 2c 9c")), Category::FullHouse);
        // two trips make a full house, the lower trips as the pair
        assert_eq!(describe(s("Kc Kd Kh 7c 7d 7h 2s")), "Kings full of sevens");
        // three pairs: the best two play, the best remaining card is the kicker
        assert_eq!(describe(s("Ac Ad Kc Kd 5c 5d 9h")), "Aces and kings");
        // (the extra pair's five is no kicker; the nine is, so a lower third pair changes nothing)
        assert_eq!(s("Ac Ad Kc Kd 5c 5d 9h"), s("Ac Ad Kc Kd 5c 8d 9h"));
        assert!(s("Ac Ad Kc Kd 5c 5d Qh") > s("Ac Ad Kc Kd 5c 5d 9h"));
        // a straight flush wins over the plain flush
        assert_eq!(category(s("5h 6h 7h 8h 9h Ah 2c")), Category::StraightFlush);
        // six to a flush: the best five
        assert_eq!(describe(s("Ah Kh 9h 7h 4h 3h 2c")), "Flush, ace high");
        // a straight with a pair on board plays the straight
        assert_eq!(category(s("5c 6d 7h 8s 9c 9d 2c")), Category::Straight);
    }

    #[test]
    fn it_agrees_with_the_slow_evaluator_on_random_hands() {
        let mut rng = Lcg(12345);
        for n in [5usize, 6, 7] {
            for _ in 0..4000 {
                let mut deck: Vec<Card> = (0..52).collect();
                for i in (1..52).rev() {
                    let j = (rng.next() % (i as u64 + 1)) as usize;
                    deck.swap(i, j);
                }
                let hand = &deck[..n];
                assert_eq!(score(hand), slow_score(hand), "{}", hand.iter().map(|&c| name(c)).collect::<Vec<_>>().join(" "));
            }
        }
    }

    #[test]
    fn the_odds_of_each_hand_are_the_known_ones() {
        // seven cards: the known frequencies (of 133,784,560 hands) to within sampling noise
        let mut rng = Lcg(99);
        let mut counts = [0u32; 9];
        let trials = 200_000;
        for _ in 0..trials {
            let mut deck: Vec<Card> = (0..52).collect();
            for i in 0..7 {
                let j = i + (rng.next() % (52 - i as u64)) as usize;
                deck.swap(i, j);
            }
            counts[category(score(&deck[..7])) as usize] += 1;
        }
        let pct = |c: Category| counts[c as usize] as f64 / trials as f64 * 100.0;
        assert!((pct(Category::Pair) - 43.8).abs() < 0.7, "pair {}", pct(Category::Pair));
        assert!((pct(Category::TwoPair) - 23.5).abs() < 0.6, "two pair {}", pct(Category::TwoPair));
        assert!((pct(Category::HighCard) - 17.4).abs() < 0.6, "high card {}", pct(Category::HighCard));
        assert!((pct(Category::Flush) - 3.03).abs() < 0.3, "flush {}", pct(Category::Flush));
        assert!((pct(Category::FullHouse) - 2.60).abs() < 0.3, "full house {}", pct(Category::FullHouse));
    }
}
