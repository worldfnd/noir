fn main(x: u8, y: pub u8) {
    let mut vector = [0; 2].as_vector();
    if x == y {
        vector = vector.push_back(y);
        vector = vector.push_back(x);
    } else {
        vector = vector.push_back(x);
    }

    vector = vector.insert(10, 100);
}
