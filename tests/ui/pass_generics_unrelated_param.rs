#[struct_cache_field::impl_cached_method]
impl<T> Hoge<T> {
    pub fn two_times_x(&self) -> u64 {
        2 * self.x
    }
}

#[struct_cache_field::add_cache_field]
struct Hoge<T> {
    x: u64,
    t: T,
}

fn main() {
    let mut hoge = Hoge {
        x: 1,
        t: "t".to_string(),
        __cache_fields__: Default::default(),
    };

    assert_eq!(hoge.two_times_x(), &2);
    assert_eq!(hoge.two_times_x(), &2);
    hoge.x = 2;
    assert_eq!(hoge.two_times_x(), &2);
}
