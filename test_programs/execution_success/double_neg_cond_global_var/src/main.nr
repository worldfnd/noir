// Regression from issue #7964 (https://github.com/noir-lang/noir/issues/7964)
global G_A: bool = false;
fn main(a: u8) -> pub u8 {
    let a = a as Field;
    if (
        (
            if (!G_A) {
                if (!G_A) {
                    (a as u128)
                } else {
                    (a as u128)
                }
            } else {
                (a as u128)
            }
                <= (a as u128)
        ) as bool
    ) {
        (-(-a)) as u8
    } else {
        (-(-a)) as u8
    }
}
