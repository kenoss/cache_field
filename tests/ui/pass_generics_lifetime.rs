#[struct_cache_field::impl_cached_method]
impl<'a> Hoge<'a> {
    pub fn two_times_x(&self) -> u64 {
        2 * self.x
    }
}

#[struct_cache_field::add_cache_field]
struct Hoge<'a> {
    x: &'a u64,
}

fn main() {
    let x = 1;
    let mut hoge = Hoge {
        x: &x,
        __cache_fields__: Default::default(),
    };

    assert_eq!(hoge.two_times_x(), &2);
    assert_eq!(hoge.two_times_x(), &2);
    let x = 2;
    hoge.x = &x;
    assert_eq!(hoge.two_times_x(), &2);
}
