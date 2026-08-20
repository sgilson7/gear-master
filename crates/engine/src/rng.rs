//! A tiny seeded PRNG.
//!
//! The engine owns its own generator rather than borrowing macroquad's global
//! one, for two reasons: the engine must not depend on a graphics crate, and a
//! run seeded with a known number has to replay identically in a test.

/// xorshift64*. Small, fast, and good enough for stocking a shop.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        // A zero state would stick at zero forever.
        Rng { state: if seed == 0 { 0x9E3779B97F4A7C15 } else { seed } }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// Uniform in `0..n`. Returns 0 when `n` is 0.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as usize
    }

    /// Fisher-Yates, so drawing without replacement is just "shuffle and take".
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        if items.len() < 2 {
            return;
        }
        for i in (1..items.len()).rev() {
            let j = self.below(i + 1);
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_sequence() {
        let a: Vec<u64> = (0..8).map(|_| Rng::new(42).next_u64()).collect();
        let mut r = Rng::new(42);
        let b: Vec<u64> = (0..8).map(|_| r.next_u64()).collect();
        assert_eq!(a[0], b[0]);
        // And two independent generators on the same seed agree throughout.
        let mut x = Rng::new(7);
        let mut y = Rng::new(7);
        for _ in 0..100 {
            assert_eq!(x.next_u64(), y.next_u64());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn below_stays_in_range() {
        let mut r = Rng::new(99);
        for n in 1..20usize {
            for _ in 0..50 {
                assert!(r.below(n) < n);
            }
        }
        assert_eq!(r.below(0), 0, "degenerate case must not divide by zero");
    }

    #[test]
    fn shuffle_keeps_every_element() {
        let mut r = Rng::new(5);
        let mut v: Vec<usize> = (0..20).collect();
        r.shuffle(&mut v);
        let mut sorted = v.clone();
        sorted.sort();
        assert_eq!(sorted, (0..20).collect::<Vec<_>>());
        assert_ne!(v, sorted, "and actually reorders them");
    }
}
