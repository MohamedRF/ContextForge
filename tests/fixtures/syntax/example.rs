use std::fmt;
pub mod garden {
    pub struct Plant;
    pub enum Color { Green }
    pub trait Grow { fn grow(&self); }
    impl Plant {
        pub fn grow(&self) { water(); }
    }
    pub const SIZE: usize = 2;
    fn water() {}
}
