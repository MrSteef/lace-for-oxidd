pub struct Lfsr {
    state: u64,
}
impl Lfsr {
    pub fn new(mut seed: usize) -> Self {
        seed += 123;
        seed *= 0xdeadbeef;
        if seed == 0 {
            seed = 1;
        }
        Lfsr { state: seed as u64 }
    }
    pub fn tick(&mut self) {
        self.state = if (self.state & 1) != 0 {
            (self.state >> 1) ^ 0x800000000000000D
        } else {
            self.state >> 1
        }
    }
    pub fn next(&mut self, bound: usize) -> usize {
        // shift in enough new bits to get a random value below the bound
        let mut tmp = bound;
        while tmp != 0 {
            self.tick();
            tmp >>= 1;
        }
        (self.state as usize) % bound
    }
}
