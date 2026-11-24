use std::convert::{TryFrom, UnsafeFrom};

fn main(small: u32, signed: i8) -> pub u8 {
    // Every `u32` is a field element, and comes back out unchanged.
    let element = Field::try_from(small).unwrap();
    assert(u32::try_from(element).unwrap() == small);

    // 300 does not fit a byte, so the checked conversion is `None` where the wrapping one
    // keeps the low bits.
    let narrowed = u8::try_from(element);
    assert(narrowed.is_some() == (small < 256));
    assert(u8::unsafe_from(element) == small as u8);

    // A negative value is no field element, and a `Field` past `i8::MAX` is no `i8`.
    assert(Field::try_from(signed).is_some() == (signed >= 0));
    assert(i8::try_from(element).is_some() == (small < 128));

    narrowed.unwrap_or(44)
}
