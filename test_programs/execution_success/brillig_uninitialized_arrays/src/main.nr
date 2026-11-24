use std::convert::UnsafeFrom;
fn main(x: u8, y: u8) -> pub u8 {
    let x = Field::from(x);
    let y = Field::from(y);
    // Safety: testing context
    unsafe {
        let notes = create_notes(x, y);
        u8::unsafe_from(sum_x(notes, x, y))
    }
}

fn sum_x(notes: [Field; 2], x: Field, y: Field) -> Field {
    notes[u32::unsafe_from(x)] + notes[u32::unsafe_from(y)]
}

unconstrained fn create_notes(x: Field, y: Field) -> [Field; 2] {
    [x, y]
}
