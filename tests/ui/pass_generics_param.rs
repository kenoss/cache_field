#[struct_cache_field::impl_cached_method]
impl<T> Hoge<T>
where
    T: ToString + From<String>,
{
    pub fn two_times_t(&self) -> T {
        let s = self.t.to_string();
        format!("{s}{s}").into()
    }
}

#[struct_cache_field::add_cache_field]
struct Hoge<T>
where
    T: ToString + From<String>,
 {
    x: u64,
    t: T,
}

fn main() {
    let mut hoge = Hoge {
        x: 1,
        t: "t".to_string(),
        __cache_fields__: Default::default(),
    };

    assert_eq!(hoge.two_times_t(), &"tt");
    assert_eq!(hoge.two_times_t(), &"tt");
    hoge.t = "s".to_string();
    assert_eq!(hoge.two_times_t(), &"tt");
}
