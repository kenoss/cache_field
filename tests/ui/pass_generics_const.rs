#[struct_cache_field::impl_cached_method]
impl<const N: u64> Hoge<N> {
    pub fn n_times_x(&self) -> u64 {
        N * self.x
    }
}

#[struct_cache_field::add_cache_field]
struct Hoge<const N: u64> {
    x: u64,
}

fn main() {
    let mut hoge = Hoge::<2> {
        x: 1,
        __cache_fields__: Default::default(),
    };

    assert_eq!(hoge.n_times_x(), &2);
    assert_eq!(hoge.n_times_x(), &2);
    hoge.x = 2;
    assert_eq!(hoge.n_times_x(), &2);
}
