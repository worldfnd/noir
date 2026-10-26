global G_A: [Field] = [].as_vector();
fn main(a: bool, b: bool, c: [u8; 32]) -> pub ([u8; 32], [u8; 32]) {
    if a {
        (
            {
                let i = G_A;
                i[(2050918985_u32 % i.len())].to_le_bytes()
            },
            // Safety: test
            unsafe {
                Field::to_le_bytes(func_1_proxy(
                    if a {
                        if b {
                            func_1_proxy(0) as u32
                        } else {
                            766482358_u32
                        }
                    } else {
                        1568313658_u32
                    },
                ))
            },
        )
    } else {
        (
            {
                let k = G_A;
                k[(779011912_u32 % k.len())].to_le_bytes()
            }, c,
        )
    }
}

unconstrained fn func_1_proxy(_a: u32) -> Field {
    0
}

#[test(should_fail_with = "attempt to calculate the remainder with a divisor of zero")]
fn fuzz_main(a: bool, b: bool) -> ([u8; 32], [u8; 32]) {
    main(
        a,
        b,
        0x08c9c54133258070e36902247f2c89f746736c2abeaba3283c66587b49e9b512.to_le_bytes(),
    )
}
