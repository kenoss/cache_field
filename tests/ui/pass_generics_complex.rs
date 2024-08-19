#[struct_cache_field::impl_cached_method]
impl<'a, T, const N: usize> Hoge<'a, T, N>
where
    T: ToString + From<String>,
{
    pub fn n_times_t(&self) -> T {
        self.t.to_string().repeat(N).into()
    }
}

#[struct_cache_field::add_cache_field]
struct Hoge<'a, T, const N: usize>
where
    T: ToString + From<String>,
{
    x: u64,
    t: &'a T,
}

fn main() {
    let t = "t".to_string();
    let mut hoge = Hoge::<String, 3> {
        x: 1,
        t: &t,
        __cache_fields__: Default::default(),
    };

    assert_eq!(hoge.n_times_t(), &"ttt");
    assert_eq!(hoge.n_times_t(), &"ttt");
    let s = "s".to_string();
    hoge.t = &s;
    assert_eq!(hoge.n_times_t(), &"ttt");
}
