contract Main {
    fn double(x: u32) -> pub u32 { x * 2 }
    fn triple(x: u32) -> pub u32 { x * 3 }
    fn quadruple(x: u32) -> pub u32 { double(double(x)) }
}
