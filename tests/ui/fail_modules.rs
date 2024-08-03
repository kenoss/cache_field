mod module1 {
    #[cache_field::impl_cached_method]
    impl Hoge {
        pub fn two_times_x(&self) -> u64 {
            2 * self.x
        }
    }

    #[cache_field::add_cache_field]
    pub struct Hoge {
        pub x: u64,
    }
}

mod module2 {
    #[cache_field::impl_cached_method]
    impl Hoge {
        pub fn three_times_y(&self) -> u64 {
            2 * self.y
        }
    }

    #[cache_field::add_cache_field]
    pub struct Hoge {
        pub y: u64,
    }
}

fn main() {}
