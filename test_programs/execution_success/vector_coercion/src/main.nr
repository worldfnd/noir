struct Hasher {
    fields: [Field],
}

impl Hasher {
    pub fn new() -> Self {
        Self { fields: [] }
    }

    pub fn add(&mut self, field: Field) {
        self.fields = self.fields.push_back(field);
    }
}

fn main(expected: pub u8, first: u8) {
    let mut hasher = Hasher::new();
    hasher.add(first as Field);
    assert(hasher.fields[0] == expected as Field);

    regression_4967();
}

fn regression_4967() {
    let var1: [(i32, u8)] = [(1, 2)];
    assert(var1.len() == 1);
    std::println(var1);
}
