//! The provably fair deck.
//!
//! Every hand's 52 cards are shuffled from two seeds, and the shuffle is committed to *before* anyone
//! acts, then shown *after* the hand so that anyone can check it:
//!
//! 1. Before the hand the table picks a secret **server seed** (32 random bytes from the operating
//!    system's randomness, mixed with the clock) and shows its **commitment**:
//!    `SHA-256("CPUNITY-POKER-COMMIT|v1|" + server seed)`. From then on the seed cannot be changed
//!    without the commitment no longer matching.
//! 2. You have a **client seed** (any text; a random one is picked for you, and you can change it
//!    whenever you like between hands). The table cannot know it in advance.
//! 3. The deck is a function of the three: `material = server seed (32 bytes) + "|" + client seed +
//!    "|" + hand number (as decimal text)`. A stream of random numbers is made from it: block `k`
//!    is `SHA-256(material + "|" + k as decimal text)` (k = 0, 1, 2 ...), read as 32-bit big-endian
//!    numbers, eight to a block, in order. The cards 0 to 51 (2c, 3c ... Ac, 2d ... As) are shuffled with
//!    Fisher-Yates: for `i` from 51 down to 1, pick `j` between 0 and `i` (both included) from the
//!    stream, and swap card `i` and card `j`. A number is thrown away and the next one used when it
//!    falls in the uneven part at the top of the range (so there is no modulo bias).
//! 4. The cards are dealt from the top of that deck in order: the i-th player (counting from the one
//!    after the dealer button) gets cards `2i` and `2i + 1`; with `n` players, card `2n` is burned, the
//!    flop is the next three, one more is burned, the turn, one burned, the river.
//! 5. After the hand the server seed is revealed. You can check that its commitment matches, that
//!    it, your client seed and the hand number give exactly the deck that was dealt, and that this
//!    deck is a real deck of 52 different cards. The computer players are only ever given their own
//!    two cards and what is on the table (see `poker_table::View`), never the deck.
//!
//! The same recipe is written in a few lines of any language: `poker_fair::tests` holds a test vector
//! that was worked out by an independent implementation of it.

use crate::poker_sha::{hex, sha256, Sha256};

pub const COMMIT_TAG: &[u8] = b"CPUNITY-POKER-COMMIT|v1|";

/// The 52 cards after the shuffle, top card first.
pub type Deck = [u8; 52];

/// What the table publishes before the hand.
pub fn commit(server_seed: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(COMMIT_TAG);
    h.update(server_seed);
    h.finalize()
}

/// The stream of numbers a shuffle draws from.
pub struct Stream {
    material: Vec<u8>,
    block_no: u64,
    block: [u8; 32],
    pos: usize,
}

impl Stream {
    pub fn new(server_seed: &[u8; 32], client_seed: &str, hand_no: u64) -> Self {
        let mut material = Vec::with_capacity(48 + client_seed.len());
        material.extend_from_slice(server_seed);
        material.push(b'|');
        material.extend_from_slice(client_seed.as_bytes());
        material.push(b'|');
        material.extend_from_slice(hand_no.to_string().as_bytes());
        let mut s = Stream { material, block_no: 0, block: [0; 32], pos: 32 };
        s.refill();
        s
    }

    fn refill(&mut self) {
        let mut h = Sha256::new();
        h.update(&self.material);
        h.update(b"|");
        h.update(self.block_no.to_string().as_bytes());
        self.block = h.finalize();
        self.block_no += 1;
        self.pos = 0;
    }

    pub fn next_u32(&mut self) -> u32 {
        if self.pos >= 32 {
            self.refill();
        }
        let b = &self.block[self.pos..self.pos + 4];
        self.pos += 4;
        u32::from_be_bytes([b[0], b[1], b[2], b[3]])
    }

    /// A number from 0 to `n - 1`, every one equally likely.
    pub fn below(&mut self, n: u32) -> u32 {
        debug_assert!(n > 0);
        // the largest multiple of n that fits in 2^32: numbers at or above it are not used
        let zone = (1u64 << 32) / u64::from(n) * u64::from(n);
        loop {
            let v = u64::from(self.next_u32());
            if v < zone {
                return (v % u64::from(n)) as u32;
            }
        }
    }
}

/// The deck for these seeds and this hand.
pub fn deck_for(server_seed: &[u8; 32], client_seed: &str, hand_no: u64) -> Deck {
    let mut stream = Stream::new(server_seed, client_seed, hand_no);
    let mut deck = [0u8; 52];
    for (i, c) in deck.iter_mut().enumerate() {
        *c = i as u8;
    }
    for i in (1..52usize).rev() {
        let j = stream.below(i as u32 + 1) as usize;
        deck.swap(i, j);
    }
    deck
}

/// Fresh randomness: the operating system's (through the keyed hashers the standard library seeds
/// from it), the clock and a counter, all mixed through SHA-256.
pub fn entropy32() -> [u8; 32] {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut pool = Sha256::new();
    for i in 0..8u64 {
        let mut h = RandomState::new().build_hasher();
        h.write_u64(i ^ COUNTER.fetch_add(1, Ordering::Relaxed));
        pool.update(&h.finish().to_le_bytes());
    }
    if let Ok(d) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        pool.update(&d.as_nanos().to_le_bytes());
    }
    let here = 0u8;
    pool.update(&(&here as *const u8 as usize).to_le_bytes());
    pool.update(format!("{:?}", std::thread::current().id()).as_bytes());
    pool.finalize()
}

/// A new secret server seed.
pub fn new_server_seed() -> [u8; 32] {
    entropy32()
}

/// A random client seed to start with (you can type your own).
pub fn random_client_seed() -> String {
    hex(&entropy32()[..8])
}

/// Everything that is shown after a hand so that it can be checked.
#[derive(Debug, Clone, PartialEq)]
pub struct HandProof {
    pub hand_no: u64,
    pub commitment: [u8; 32],
    pub server_seed: [u8; 32],
    pub client_seed: String,
    /// The deck as it was dealt.
    pub deck: Deck,
}

/// What a check found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofError {
    /// The revealed server seed is not the one that was committed to.
    CommitmentMismatch,
    /// The seeds do not make the deck that was dealt.
    DeckMismatch,
    /// The deck is not 52 different cards.
    NotADeck,
}

impl HandProof {
    pub fn new(server_seed: [u8; 32], client_seed: &str, hand_no: u64) -> Self {
        HandProof {
            hand_no,
            commitment: commit(&server_seed),
            server_seed,
            client_seed: client_seed.to_string(),
            deck: deck_for(&server_seed, client_seed, hand_no),
        }
    }

    /// Checks the whole thing from scratch.
    pub fn verify(&self) -> Result<(), ProofError> {
        if commit(&self.server_seed) != self.commitment {
            return Err(ProofError::CommitmentMismatch);
        }
        let mut seen = [false; 52];
        for &c in &self.deck {
            if c >= 52 || std::mem::replace(&mut seen[c as usize], true) {
                return Err(ProofError::NotADeck);
            }
        }
        if deck_for(&self.server_seed, &self.client_seed, self.hand_no) != self.deck {
            return Err(ProofError::DeckMismatch);
        }
        Ok(())
    }

    /// The commitment as text.
    pub fn commitment_hex(&self) -> String {
        hex(&self.commitment)
    }

    pub fn server_seed_hex(&self) -> String {
        hex(&self.server_seed)
    }

    /// The deck as text, top card first ("Ks 7d ...").
    pub fn deck_text(&self) -> String {
        self.deck.iter().map(|&c| crate::poker_cards::name(c)).collect::<Vec<_>>().join(" ")
    }
}

/// A short hash of a deck, for showing next to a hand.
pub fn deck_fingerprint(deck: &Deck) -> String {
    hex(&sha256(deck))[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_matches_an_independent_implementation() {
        // Worked out with a separate implementation of the recipe in the module's documentation
        // (a few lines of Python): seed = 32 zero bytes, client seed "abc", hand 1.
        let seed = [0u8; 32];
        assert_eq!(hex(&commit(&seed)), COMMIT_VECTOR);
        let deck = deck_for(&seed, "abc", 1);
        let text: Vec<String> = deck.iter().map(|&c| crate::poker_cards::name(c)).collect();
        assert_eq!(text.join(" "), DECK_VECTOR);
    }

    const COMMIT_VECTOR: &str = "119d91db4132ed0bc2bd2e5e96952c884b2a2faedf3636574ac4cb73fb63e224";
    const DECK_VECTOR: &str = "Ks Jd 8d Td 2s As Kc Ah 6h Qd Qh 6d 7c 4c Ts 4h 3c Jc 9h 5h Ac Kh 8h 7d Th Tc 7h 3s Ad 8c 2h 7s 3d Js 5d 2d 9c 4s 6c Qc 5c 9s Kd 5s 9d Jh Qs 2c 4d 8s 6s 3h";

    #[test]
    fn a_deck_is_a_deck_and_the_same_seeds_make_the_same_one() {
        let seed = new_server_seed();
        let a = deck_for(&seed, "x", 7);
        assert_eq!(a, deck_for(&seed, "x", 7));
        let mut sorted = a;
        sorted.sort_unstable();
        assert!(sorted.iter().enumerate().all(|(i, &c)| c as usize == i));
        // any change to any input changes the deck
        assert_ne!(a, deck_for(&seed, "y", 7));
        assert_ne!(a, deck_for(&seed, "x", 8));
        let mut other = seed;
        other[31] ^= 1;
        assert_ne!(a, deck_for(&other, "x", 7));
        // (and the order of the pieces matters: "1|23" is not "12|3")
        assert_ne!(deck_for(&seed, "1", 23), deck_for(&seed, "12", 3));
    }

    #[test]
    fn a_hand_checks_out_and_cheating_is_caught() {
        let proof = HandProof::new(new_server_seed(), "my seed", 42);
        assert_eq!(proof.verify(), Ok(()));
        // a different server seed than the one committed to
        let mut fake = proof.clone();
        fake.server_seed[0] ^= 0x55;
        assert_eq!(fake.verify(), Err(ProofError::CommitmentMismatch));
        // a deck that was swapped after the commitment
        let mut stacked = proof.clone();
        stacked.deck.swap(0, 1);
        assert_eq!(stacked.verify(), Err(ProofError::DeckMismatch));
        // a deck with a repeated card
        let mut bad = proof.clone();
        bad.deck[1] = bad.deck[0];
        assert_eq!(bad.verify(), Err(ProofError::NotADeck));
        // a changed client seed or hand number
        let mut other = proof.clone();
        other.client_seed.push('!');
        assert_eq!(other.verify(), Err(ProofError::DeckMismatch));
        let mut later = proof;
        later.hand_no += 1;
        assert_eq!(later.verify(), Err(ProofError::DeckMismatch));
    }

    #[test]
    fn below_is_even_and_stays_in_range() {
        let mut s = Stream::new(&[3; 32], "even", 1);
        for n in [1u32, 2, 3, 7, 52, 1000] {
            for _ in 0..200 {
                assert!(s.below(n) < n);
            }
        }
        // 3 does not divide 2^32, so the top of the range is thrown away: all three come up evenly
        let mut counts = [0u32; 3];
        for _ in 0..30_000 {
            counts[s.below(3) as usize] += 1;
        }
        for c in counts {
            assert!((c as i32 - 10_000).abs() < 400, "{counts:?}");
        }
    }

    #[test]
    fn every_card_is_equally_likely_in_every_place() {
        // 20,000 shuffles from fresh random seeds; count where each card lands (a 52 x 52 table of
        // about 385 each) and take the chi-square: it should be about 51 * 51 = 2601, give or take
        // 72 (one standard deviation). Anything past six deviations would be a biased shuffle.
        let n = 20_000usize;
        let mut table = vec![0u32; 52 * 52];
        for i in 0..n {
            let seed = {
                let mut s = [0u8; 32];
                s[..8].copy_from_slice(&(i as u64).to_le_bytes());
                s[8..16].copy_from_slice(&entropy32()[..8]);
                s
            };
            let deck = deck_for(&seed, "stats", i as u64);
            for (place, &c) in deck.iter().enumerate() {
                table[c as usize * 52 + place] += 1;
            }
        }
        let expected = n as f64 / 52.0;
        let chi: f64 = table.iter().map(|&o| (o as f64 - expected).powi(2) / expected).sum();
        assert!((chi - 2601.0).abs() < 6.0 * 72.2, "chi-square {chi}");
        // and nothing is missing or doubled in any row or column
        for c in 0..52 {
            assert_eq!((0..52).map(|p| table[c * 52 + p]).sum::<u32>() as usize, n);
        }
    }

    #[test]
    fn the_top_card_and_pairs_of_cards_are_fair_too() {
        // the chance that the first two cards are a pair is 3/51; both cards red is 25/102
        let n = 40_000usize;
        let (mut pairs, mut red, mut suited) = (0usize, 0usize, 0usize);
        for i in 0..n {
            let mut seed = entropy32();
            seed[0] = i as u8;
            let d = deck_for(&seed, "pairs", i as u64);
            let (a, b) = (d[0], d[1]);
            if crate::poker_cards::rank(a) == crate::poker_cards::rank(b) {
                pairs += 1;
            }
            let is_red = |c: u8| matches!(crate::poker_cards::suit(c), 1 | 2);
            if is_red(a) && is_red(b) {
                red += 1;
            }
            if crate::poker_cards::suit(a) == crate::poker_cards::suit(b) {
                suited += 1;
            }
        }
        let f = |x: usize| x as f64 / n as f64;
        assert!((f(pairs) - 3.0 / 51.0).abs() < 0.006, "pairs {}", f(pairs));
        assert!((f(red) - 26.0 * 25.0 / (52.0 * 51.0)).abs() < 0.01, "red {}", f(red));
        assert!((f(suited) - 12.0 / 51.0).abs() < 0.01, "suited {}", f(suited));
    }

    #[test]
    fn fresh_seeds_do_not_repeat() {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..500 {
            assert!(seen.insert(new_server_seed()));
        }
        assert_ne!(random_client_seed(), random_client_seed());
    }
}
