use crate::{drivers::serial::Serial, x86_64::io::Port};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UartError {
    InvalidBaudRate,
    DivisorOutOfRange,
    DeviceNotResponding,
    Timeout,
}

pub struct Uart;

impl Uart {
    // Read, DLAB = 0
    #[allow(unused)]
    const RBR: u16 = 0x00;
    // Write, DLAB = 0
    const THR: u16 = 0x00;
    // Read/write, DLAB = 1
    const DLL: u16 = 0x00;

    // Read/write, DLAB = 0
    const IER: u16 = 0x01;
    // Read/write, DLAB = 1
    const DLM: u16 = 0x01;

    // Read, DLAB = 0
    #[allow(unused)]
    const IIR: u16 = 0x02;
    // Write, DLAB = 0
    const FCR: u16 = 0x02;

    // Read/write, DLAB = X
    const LCR: u16 = 0x03;

    // Read/write, DLAB = X
    const MCR: u16 = 0x04;

    // Read
    const LSR: u16 = 0x05;

    // Read
    #[allow(unused)]
    const MSR: u16 = 0x06;

    // Read/write
    #[allow(unused)]
    const SCR: u16 = 0x07;
}

impl Serial for Uart {
    // todo: replace with UART sepcific errors
    type Error = UartError;

    fn init() -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        const COM_1_PORT: u16 = 0x3F8;

        let mut port = Port::new(COM_1_PORT + Self::IER);

        unsafe {
            // disable interrupt
            port.write(0x00);
        }

        // 1000 0000 (0x80)
        // mask bit 7 of LCR register to access DLAB : RBR, THR and IER accessible
        port = Port::new(COM_1_PORT + Self::LCR);
        unsafe {
            let mut val = port.read();
            // set bit 7 of LCR which is Divisor Latch Access Bit
            val |= 1u8 << 7;
            port.write(val);
        }

        // sets baud rate to 115200
        // 1_843_200/(16 * 1) = 115_200
        port = Port::new(COM_1_PORT + Self::DLL);
        unsafe {
            port.write(0x01);
        }

        port = Port::new(COM_1_PORT + Self::DLM);
        unsafe {
            port.write(0x00);
        }

        port = Port::new(COM_1_PORT + Self::LCR);

        unsafe {
            // standard 8N1 configuration
            // sets Bit 1:0 -> 8 bits data
            // Bit 2 = 0 -> 1 stop bit
            // Bit 4:3 = 00 -> parity disabled
            // Bit 7 = 0 -> DLAB disabled
            port.write(0x03);
        }

        port = Port::new(COM_1_PORT + Self::FCR);
        unsafe {
            // set bit 0 -> Enable FIFO
            // set bit 1:2 -> clears Transmit FIFO and Receive FIFO
            // set bit 7:6 -> Interrupt Trigger Level we set to 14

            // 1100_0111 = 0xC7
            port.write(0x01);
        }

        port = Port::new(COM_1_PORT + Self::MCR);

        unsafe {
            // IRQs enabled, RTS/DSR clear
            port.write(0x0B);
            // set in loopback mode
            port.write(0x1E);
        }

        // test
        port = Port::new(COM_1_PORT + Self::THR);

        unsafe {
            port.write(0xAB);
        }

        let val = unsafe { port.read() };

        if val != 0xAB {
            return Err(UartError::DeviceNotResponding);
        }

        port = Port::new(COM_1_PORT + Self::MCR);

        unsafe {
            // (not-loopback with IRQs enabled and OUT#1 and OUT#2 bits enabled)
            port.write(0x0F);
        }

        Ok(Self)
    }

    fn read_byte(&self) -> u8 {
        let lsr = Port::new(0x3F8u16 + Self::LSR);

        let val = unsafe {
            // bit 0 is Data ready (DR)
            // Set if there is data that can be read
            while lsr.read() & 0x01 == 0 {}

            lsr.read()
        };

        return val;
    }

    fn write_byte(&mut self, byte: u8) {
        let lsr = Port::new(0x3F8u16 + Self::LSR);

        unsafe {
            // bit 5 is Transmitter holding register empty (THRE)
            // Set if the transmission buffer is empty (i.e. data can be sent)
            while lsr.read() & 0x20 == 0 {}
            Port::new(0x3F8u16).write(byte);
        }
    }
}
