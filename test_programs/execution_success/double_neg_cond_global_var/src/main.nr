use std::convert::UnsafeFrom;
// Regression from issue #7964 (https://github.com/noir-lang/noir/issues/7964)
global G_A: bool = false;
fn main(a: u8) -> pub u8 {
    let a = Field::from(a);
    if (
        (
            if (!G_A) {
                if (!G_A) {
                    (u128::unsafe_from(a))
                } else {
                    (u128::unsafe_from(a))
                }
            } else {
                (u128::unsafe_from(a))
            }
                <= (u128::unsafe_from(a))
        ) as bool
    ) {
        u8::unsafe_from(-(-a))
    } else {
        u8::unsafe_from(-(-a))
    }
}
