pub trait AsMubi {
    fn as_mubi(&self) -> u32;
}

impl AsMubi for bool {
    fn as_mubi(&self) -> u32 {
        if *self {
            6
        } else {
            9
        }
    }
}
