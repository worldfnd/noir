fn main(x: u8, y: u8) -> pub u8 {
    // Safety: testing context
    unsafe {
        let notes = create_notes(x as Field, y as Field);
        sum_x(notes, x as Field, y as Field) as u8
    }
}

fn sum_x(notes: [Field; 2], x: Field, y: Field) -> Field {
    notes[x as u32] + notes[y as u32]
}

unconstrained fn create_notes(x: Field, y: Field) -> [Field; 2] {
    [x, y]
}
