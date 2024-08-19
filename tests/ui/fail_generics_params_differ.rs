#[struct_cache_field::impl_cached_method]
impl<T> Hoge<T>
where
    T: ToString + From<String>,
{
    pub fn two_times_x(&self) -> T {
        let s = self.x.to_string();
        format!("{s}{s}").into()
    }
}

#[struct_cache_field::add_cache_field]
struct Hoge<S>
where
    S: ToString + From<String>,
{
    x: u64,
    t: S,
}

fn main() {}
