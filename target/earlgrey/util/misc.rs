use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unalign};

#[derive(Clone, Copy, FromBytes, Immutable, IntoBytes, KnownLayout)]
pub struct UnalignedU64(Unalign<u64>);

impl UnalignedU64 {
    pub fn get(&self) -> u64 {
        self.0.get()
    }
    pub fn set(&mut self, v: u64) {
        self.0.set(v)
    }
}

impl ufmt::uDisplay for UnalignedU64 {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
    {
        let v = self.get();
        ufmt::uwrite!(f, "{:016x}", v)
    }
}

impl ufmt::uDebug for UnalignedU64 {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
    {
        ufmt::uDisplay::fmt(self, f)
    }
}
