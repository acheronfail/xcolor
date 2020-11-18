/// TODO: doc
pub trait EnsureOdd {
    fn ensure_odd(self) -> Self;
}

macro_rules! impl_ensure_odd {
    ($type:ident) => {
        impl EnsureOdd for $type {
            /// TODO: doc
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
impl_ensure_odd!(isize);
impl_ensure_odd!(usize);
