pub trait EnsureOdd {
    fn ensure_odd(self) -> Self;
}

macro_rules! impl_ensure_odd {
    ($type:ident) => {
        impl EnsureOdd for $type {
            fn ensure_odd(self) -> Self {
                if self % 2 == 0 {
                    self + 1
                } else {
                    self
                }
            }
        }
    };
}

impl_ensure_odd!(u16);
impl_ensure_odd!(u32);
impl_ensure_odd!(usize);

pub trait Clamped {
    fn clamped(self, min: Self, max: Self) -> Self;
}

macro_rules! impl_clamped {
    ($type:ident) => {
        impl Clamped for $type {
            fn clamped(self, min: Self, max: Self) -> Self {
                if self < min {
                    min
                } else if self > max {
                    max
                } else {
                    self
                }
            }
        }
    };
}

impl_clamped!(i16);

