// docs:start:not-trait
pub trait Not {
    fn not(self: Self) -> Self;
}
// docs:end:not-trait

// docs:start:not-trait-impls
impl Not for bool {
    fn not(self) -> bool {
        !self
    }
}

impl<let N: u32> Not for u<N> {
    fn not(self) -> Self {
        !self
    }
}

impl<let N: u32> Not for i<N> {
    fn not(self) -> Self {
        !self
    }
}

// docs:end:not-trait-impls

// docs:start:bitor-trait
pub trait BitOr {
    fn bitor(self, other: Self) -> Self;
}
// docs:end:bitor-trait

impl BitOr for bool {
    fn bitor(self, other: bool) -> bool {
        self | other
    }
}

impl<let N: u32> BitOr for u<N> {
    fn bitor(self, other: Self) -> Self {
        self | other
    }
}

impl<let N: u32> BitOr for i<N> {
    fn bitor(self, other: Self) -> Self {
        self | other
    }
}

// docs:start:bitand-trait
pub trait BitAnd {
    fn bitand(self, other: Self) -> Self;
}
// docs:end:bitand-trait

impl BitAnd for bool {
    fn bitand(self, other: bool) -> bool {
        self & other
    }
}

impl<let N: u32> BitAnd for u<N> {
    fn bitand(self, other: Self) -> Self {
        self & other
    }
}

impl<let N: u32> BitAnd for i<N> {
    fn bitand(self, other: Self) -> Self {
        self & other
    }
}

// docs:start:bitxor-trait
pub trait BitXor {
    fn bitxor(self, other: Self) -> Self;
}
// docs:end:bitxor-trait

impl BitXor for bool {
    fn bitxor(self, other: bool) -> bool {
        self ^ other
    }
}

impl<let N: u32> BitXor for u<N> {
    fn bitxor(self, other: Self) -> Self {
        self ^ other
    }
}

impl<let N: u32> BitXor for i<N> {
    fn bitxor(self, other: Self) -> Self {
        self ^ other
    }
}

// docs:start:shl-trait
pub trait Shl {
    fn shl(self, other: Self) -> Self;
}
// docs:end:shl-trait

impl<let N: u32> Shl for u<N> {
    fn shl(self, other: Self) -> Self {
        self << other
    }
}

impl<let N: u32> Shl for i<N> {
    fn shl(self, other: Self) -> Self {
        self << other
    }
}

// docs:start:shr-trait
pub trait Shr {
    fn shr(self, other: Self) -> Self;
}
// docs:end:shr-trait

impl<let N: u32> Shr for u<N> {
    fn shr(self, other: Self) -> Self {
        self >> other
    }
}

impl<let N: u32> Shr for i<N> {
    fn shr(self, other: Self) -> Self {
        self >> other
    }
}

mod tests {
    use crate::ops::bit::{BitAnd, BitOr, BitXor, Not, Shl, Shr};

    #[test]
    fn not() {
        assert_eq(false.not(), true);
        assert_eq(true.not(), false);
        assert_eq(0_u8.not(), 255);
        assert_eq(0_u16.not(), 65535);
        assert_eq(0_u32.not(), 4294967295);
        assert_eq(0_u64.not(), 18446744073709551615);
        assert_eq(0_u128.not(), 340282366920938463463374607431768211455);
        assert_eq(0_i8.not(), -1);
        assert_eq(0_i16.not(), -1);
        assert_eq(0_i32.not(), -1);
        assert_eq(0_i64.not(), -1);
    }

    #[test]
    fn bitor() {
        assert_eq(true.bitor(false), true);
        assert_eq(1_u8.bitor(2), 3);
        assert_eq(1_u16.bitor(2), 3);
        assert_eq(1_u32.bitor(2), 3);
        assert_eq(1_u64.bitor(2), 3);
        assert_eq(1_u128.bitor(2), 3);
        assert_eq(1_i8.bitor(2), 3);
        assert_eq(1_i16.bitor(2), 3);
        assert_eq(1_i32.bitor(2), 3);
        assert_eq(1_i64.bitor(2), 3);
    }

    #[test]
    fn bitand() {
        assert_eq(true.bitand(false), false);
        assert_eq(3_u8.bitand(5), 1);
        assert_eq(3_u16.bitand(5), 1);
        assert_eq(3_u32.bitand(5), 1);
        assert_eq(3_u64.bitand(5), 1);
        assert_eq(3_u128.bitand(5), 1);
        assert_eq(3_i8.bitand(5), 1);
        assert_eq(3_i16.bitand(5), 1);
        assert_eq(3_i32.bitand(5), 1);
        assert_eq(3_i64.bitand(5), 1);
    }

    #[test]
    fn bitxor() {
        assert_eq(true.bitxor(false), true);
        assert_eq(3_u8.bitxor(5), 6);
        assert_eq(3_u16.bitxor(5), 6);
        assert_eq(3_u32.bitxor(5), 6);
        assert_eq(3_u64.bitxor(5), 6);
        assert_eq(3_u128.bitxor(5), 6);
        assert_eq(3_i8.bitxor(5), 6);
        assert_eq(3_i16.bitxor(5), 6);
        assert_eq(3_i32.bitxor(5), 6);
        assert_eq(3_i64.bitxor(5), 6);
    }

    #[test]
    fn shl() {
        assert_eq(1_u8.shl(1), 2);
        assert_eq(1_u16.shl(1), 2);
        assert_eq(1_u32.shl(1), 2);
        assert_eq(1_u64.shl(1), 2);
        assert_eq(1_u128.shl(1), 2);
        assert_eq(1_i8.shl(1), 2);
        assert_eq(1_i16.shl(1), 2);
        assert_eq(1_i32.shl(1), 2);
        assert_eq(1_i64.shl(1), 2);
    }

    #[test]
    fn shr() {
        assert_eq(4_u8.shr(1), 2);
        assert_eq(4_u16.shr(1), 2);
        assert_eq(4_u32.shr(1), 2);
        assert_eq(4_u64.shr(1), 2);
        assert_eq(4_u128.shr(1), 2);
        assert_eq(4_i8.shr(1), 2);
        assert_eq(4_i16.shr(1), 2);
        assert_eq(4_i32.shr(1), 2);
        assert_eq(4_i64.shr(1), 2);
    }
}
