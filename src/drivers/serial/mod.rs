pub mod uart16550;

pub trait Serial {
    type Error;

    fn init() -> Result<Self, Self::Error>
    where
        Self: Sized;

    fn write_byte(&mut self, byte: u8);
    fn read_byte(&self) -> u8;
}

pub struct SerialWriter<T: Serial>(pub T);

impl<T: Serial> core::fmt::Write for SerialWriter<T> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for b in s.bytes() {
            self.0.write_byte(b);
        }
        Ok(())
    }
}
