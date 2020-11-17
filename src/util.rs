pub trait EnsureOdd {
    fn ensure_odd(self) -> Self;
}

impl EnsureOdd for u16 {
    fn ensure_odd(self) -> Self {
        if self % 2 == 0 {
            self + 1
        } else {
            self
        }
    }
}

impl EnsureOdd for usize {
    fn ensure_odd(self) -> Self {
        if self % 2 == 0 {
            self + 1
        } else {
            self
        }
    }
}
