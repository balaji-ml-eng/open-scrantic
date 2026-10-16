//! The original additive lagged-Fibonacci generator (seg4:1e86), seeded from SCRANTIC.EXE.

pub struct Rng {
    table: [u16; 56],
    j: usize,
    k: usize,
}

impl Rng {
    pub fn new(table: [u16; 56], j: usize, k: usize) -> Self {
        Self {
            table,
            j: j % 56,
            k: k % 56,
        }
    }

    /// Startup warm-up: the original discards `time() & 0xFFFF` values.
    pub fn warm_up(&mut self, unix_time: u64) {
        for _ in 0..(unix_time & 0xFFFF) {
            self.next_u16();
        }
    }

    pub fn next_u16(&mut self) -> u16 {
        let v = self.table[self.k].wrapping_add(self.table[self.j]);
        self.table[self.k] = v;
        self.k = (self.k + 1) % 56;
        self.j = (self.j + 1) % 56;
        v
    }

    /// `rand() % n` as the original computes it (unsigned 16-bit).
    pub fn below(&mut self, n: u16) -> u16 {
        if n == 0 { 0 } else { self.next_u16() % n }
    }

    /// Percentage-weighted pick (seg4:1f1b): returns the 1-based index of the chosen weight,
    /// or 0 when the draw falls past all weights.
    pub fn pick(&mut self, weights: &[u16]) -> usize {
        let mut r = self.next_u16();
        for (i, &w) in weights.iter().enumerate() {
            let threshold = w.wrapping_mul(655);
            if threshold > r {
                return i + 1;
            }
            r -= threshold;
        }
        0
    }

    pub fn chance(&mut self, percent: u16) -> bool {
        self.pick(&[percent]) == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn additive_lag_and_wraparound() {
        let mut table = [0u16; 56];
        table[24] = 1;
        table[55] = 0xFFFF;
        let mut rng = Rng::new(table, 24, 55);
        assert_eq!(rng.next_u16(), 0); // 0xFFFF + 1 wraps
        assert_eq!(rng.next_u16(), 0); // k=0, j=25
        assert_eq!((rng.j, rng.k), (26, 1));
    }

    #[test]
    fn pick_uses_percent_thresholds() {
        let mut rng = Rng::new([0; 56], 24, 55);
        assert_eq!(rng.pick(&[50]), 1); // draw is 0
        assert!(rng.chance(1));
    }
}
