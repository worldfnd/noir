// `main` returns no `Field`, even inside a struct.
struct Point {
    x: u32,
    y: Field,
}

fn main(x: u32) -> pub Point {
    Point { x, y: 1 }
}
