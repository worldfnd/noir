fn main(flag: bool, arr: [u8; 4]) {
    let sli = arr.as_vector();
    println(sli);
    std::hint::black_box(arr);
}
