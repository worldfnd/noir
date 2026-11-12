global G_A: u16 = 63895;
fn main() -> pub u16 {
    {
        let mut a = G_A;
        a = {
            for idx_e in 28603..28607 {
                a = a + G_A;
                let _f = (
                    (idx_e as Field), if (!if true { true } else { true }) {
                        ""
                    } else {
                        ""
                    },
                );
            }
            G_A
        };
        64427
    }
}
