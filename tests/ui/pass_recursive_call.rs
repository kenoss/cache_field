#[struct_cache_field::impl_cached_method]
impl Hoge {
    pub fn two_times_x(&self) -> u64 {
        2 * self.x
    }

    pub fn two_times_x_plus_y(&mut self) -> u64 {
        self.two_times_x() + self.y
    }
}

#[struct_cache_field::add_cache_field]
struct Hoge {
    x: u64,
    y: u64,
}

fn main() {
    let mut hoge = Hoge {
        x: 1,
        y: 10,
        __cache_fields__: Default::default(),
    };

    assert_eq!(hoge.two_times_x_plus_y(), &12);
    assert_eq!(hoge.two_times_x_plus_y(), &12);
    hoge.x = 2;
    hoge.y = 20;
    assert_eq!(hoge.two_times_x_plus_y(), &12);
}
