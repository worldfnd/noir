impl<let N: u32> u<N> {
    /// Returns the maximum value representable by `u<N>`: `2^N - 1`.
    pub fn max_value() -> Self {
        !(0 as Self)
    }

    /// Returns the minimum value representable by `u<N>`: 0.
    pub fn min_value() -> Self {
        0
    }

    /// Returns the number of bits in `u<N>`: `N`.
    pub fn bits() -> u32 {
        N
    }
}

impl<let N: u32> i<N> {
    /// Returns the maximum value representable by `i<N>`: `2^(N-1) - 1`.
    pub fn max_value() -> Self {
        // Avoid constructing 2^(N-1), which exceeds the signed range.
        let half = (1 as Self) << ((N - 2) as Self);
        half - 1 + half
    }

    /// Returns the minimum value representable by `i<N>`: `-2^(N-1)`.
    pub fn min_value() -> Self {
        let half = (1 as Self) << ((N - 2) as Self);
        -half - half
    }

    /// Returns the number of bits in `i<N>`: `N`.
    pub fn bits() -> u32 {
        N
    }
}

mod tests {
    #[test]
    fn test_u8_limits() {
        assert_eq(u8::max_value(), 255);
        assert_eq(u8::min_value(), 0);
        assert_eq(u8::bits(), 8);
    }

    #[test]
    fn test_u16_limits() {
        assert_eq(u16::max_value(), 65535);
        assert_eq(u16::min_value(), 0);
        assert_eq(u16::bits(), 16);
    }

    #[test]
    fn test_u32_limits() {
        assert_eq(u32::max_value(), 4294967295);
        assert_eq(u32::min_value(), 0);
        assert_eq(u32::bits(), 32);
    }

    #[test]
    fn test_u64_limits() {
        assert_eq(u64::max_value(), 18446744073709551615);
        assert_eq(u64::min_value(), 0);
        assert_eq(u64::bits(), 64);
    }

    #[test]
    fn test_u128_limits() {
        assert_eq(u128::max_value(), 340282366920938463463374607431768211455);
        assert_eq(u128::min_value(), 0);
        assert_eq(u128::bits(), 128);
    }

    #[test]
    fn test_i8_limits() {
        assert_eq(i8::max_value(), 127);
        assert_eq(i8::min_value(), -128);
        assert_eq(i8::bits(), 8);
    }

    #[test]
    fn test_i16_limits() {
        assert_eq(i16::max_value(), 32767);
        assert_eq(i16::min_value(), -32768);
        assert_eq(i16::bits(), 16);
    }

    #[test]
    fn test_i32_limits() {
        assert_eq(i32::max_value(), 2147483647);
        assert_eq(i32::min_value(), -2147483648);
        assert_eq(i32::bits(), 32);
    }

    #[test]
    fn test_i64_limits() {
        assert_eq(i64::max_value(), 9223372036854775807);
        assert_eq(i64::min_value(), -9223372036854775808);
        assert_eq(i64::bits(), 64);
    }
}
