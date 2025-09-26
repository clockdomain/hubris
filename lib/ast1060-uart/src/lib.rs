// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![no_std]

pub use embedded_hal::serial::{Read, Write};
use ast1060_pac as device;
use unwrap_lite::UnwrapLite;
pub use embedded_io::{Read as IoRead, Write as IoWrite};

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Error {
    Frame,
    Parity,
    Noise,
    BufFull,
}

pub enum InterruptDecoding {
    ModemStatusChange = 0,
    TxEmpty = 1,
    RxDataAvailable = 2,
    LineStatusChange = 3,
    CharacterTimeout = 6,
    Unknown = -1,
}

impl TryFrom<u8> for InterruptDecoding {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value & 0x07 {
            0 => Ok(InterruptDecoding::ModemStatusChange),
            1 => Ok(InterruptDecoding::TxEmpty),
            2 => Ok(InterruptDecoding::RxDataAvailable),
            3 => Ok(InterruptDecoding::LineStatusChange),
            6 => Ok(InterruptDecoding::CharacterTimeout),
            _ => Err(()),
        }
    }
}

pub struct Usart<'a> {
    usart: &'a device::uart::RegisterBlock,
}

impl<'a> From<&'a device::uart::RegisterBlock> for Usart<'a> {
    // this function assumes that all necessary configuration of the syscon,
    // flexcomm and gpio have been done
    fn from(usart: &'a device::uart::RegisterBlock) -> Self {
        unsafe {
            usart.uartfcr().write(|w| {
                w.enbl_uartfifo().set_bit();
                w.rx_fiforst().set_bit();
                w.tx_fiforst().set_bit();
                w.define_the_rxr_fifointtrigger_level().bits(0b10) // Example trigger level
            });
        }

        Self { usart }
            .set_rate(Rate::MBaud1_5)
            .set_8n1()
            .interrupt_enable()
    }
}

impl embedded_io::ErrorType for Usart<'_> {
    type Error = Error;
}

impl embedded_io::Error for Error {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::Other
    }
}

// embedded-io implementation for modern async interfaces
impl IoWrite for Usart<'_> {
    fn flush(&mut self) -> Result<(), Error> {
        while !self.is_tx_idle() {}
        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        let mut counter = 0;
        for byte in buf {
            if !self.is_tx_full() {
                self.usart
                    .uartthr()
                    .write(|w| unsafe { w.bits(*byte as u32) });
                counter += 1;
            } else {
                break;
            }
        }
        Ok(counter)
    }
}

impl IoRead for Usart<'_> {
    fn read(&mut self, out: &mut [u8]) -> Result<usize, Self::Error> {
        let mut count = 0;
        while !self.is_rx_empty() && count < out.len() {
            let byte = self.usart.uartrbr().read().bits() as u8;
            if self.is_rx_frame_err() {
                return Err(Error::Frame);
            } else if self.is_rx_parity_err() {
                return Err(Error::Parity);
            } else if self.is_rx_noise_err() {
                return Err(Error::Noise);
            }

            out[count] = byte;
            count += 1;
        }
        Ok(count)
    }
}

// embedded-hal implementation for nb interfaces
impl Write<u8> for Usart<'_> {
    type Error = Error;

    fn flush(&mut self) -> nb::Result<(), Error> {
        if self.is_tx_idle() {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }

    fn write(&mut self, byte: u8) -> nb::Result<(), Error> {
        if !self.is_tx_full() {
            self.usart.uartthr().write(|w| unsafe { w.bits(byte as u32) });
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl Read<u8> for Usart<'_> {
    type Error = Error;

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        if !self.is_rx_empty() {
            let byte = self.usart.uartrbr().read().bits() as u8;
            if self.is_rx_frame_err() {
                Err(nb::Error::Other(Error::Frame))
            } else if self.is_rx_parity_err() {
                Err(nb::Error::Other(Error::Parity))
            } else if self.is_rx_noise_err() {
                Err(nb::Error::Other(Error::Noise))
            } else {
                Ok(byte.try_into().unwrap_lite())
            }
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

pub enum Rate {
    Baud9600,
    Baud19200,
    MBaud1_5,
}

impl<'a> Usart<'a> {
    pub fn set_rate(self, rate: Rate) -> Self {
        // These baud rates assume that the uart clock is set to 24Mhz.
        // Enable DLAB to access divisor latch registers
        self.usart.uartlcr().modify(|_, w| w.dlab().set_bit());

        // Divisor = 24M / (13 * 16 * Baud Rate)
        match rate {
            Rate::Baud9600 => {
                self.usart.uartdlh().write(|w| unsafe { w.bits(0) });
                self.usart.uartdll().write(|w| unsafe { w.bits(12) });
            }
            Rate::Baud19200 => {
                self.usart.uartdlh().write(|w| unsafe { w.bits(0) });
                self.usart.uartdll().write(|w| unsafe { w.bits(6) });
            }
            Rate::MBaud1_5 => {
                self.usart.uartdlh().write(|w| unsafe { w.bits(0) });
                self.usart.uartdll().write(|w| unsafe { w.bits(1) });
            }
        }
        // Disable DLAB to access other registers
        self.usart.uartlcr().modify(|_, w| w.dlab().clear_bit());

        self
    }

    pub fn interrupt_enable(self) -> Self {
        self.usart.uartier().write(|w| {
            w.erbfi().set_bit(); // Enable Received Data Available Interrupt
            // w.etbei().set_bit(); // Enable Transmitter Holding Register Empty Interrupt
            // w.elsi().set_bit(); // Enable Receiver Line Status Interrupt
            // w.edssi().set_bit() // Enable Modem Status Interrupt
            w
        });

        self
    }

    pub fn set_8n1(self) -> Self {
        // Configure 8N1: 8 data bits, no parity, 1 stop bit
        // self.usart.uartlcr().write( |w| {
        // });
        self
    }

    pub fn is_tx_full(&self) -> bool {
        !self.usart.uartlsr().read().thre().bit()
    }

    pub fn is_rx_empty(&self) -> bool {
        !self.usart.uartlsr().read().dr().bit()
    }

    pub fn is_rx_frame_err(&self) -> bool {
        self.usart.uartlsr().read().fe().bit_is_set()
    }

    pub fn is_rx_parity_err(&self) -> bool {
        self.usart.uartlsr().read().pe().bit_is_set()
    }

    pub fn is_rx_noise_err(&self) -> bool {
        // self.usart.uartlsr().read().rxnoise().bit()
        false
    }

    pub fn read_interrupt_status(&self) -> InterruptDecoding {
        InterruptDecoding::try_from(
            self.usart.uartiir().read().intdecoding_table().bits() & 0x07,
        )
        .unwrap_or(InterruptDecoding::Unknown)
    }

    pub fn read_interrupt_status_raw(&self) -> u8 {
        self.usart.uartiir().read().intdecoding_table().bits() & 0x07
    }

    pub fn read_line_status(&self) -> u8 {
        self.usart.uartlsr().read().bits() as u8
    }

    pub fn read_modem_status(&self) -> u8 {
        self.usart.uartmsr().read().bits() as u8
    }

    pub fn is_tx_idle(&self) -> bool {
        // self.usart.uartlsr().read().txter_empty().bit_is_set()
        // self.usart.uartlsr().read().txter_empty().bit_is_set()
        self.usart.uartiir().read().intdecoding_table() == 0x01
    }

    pub fn set_tx_idle_interrupt(&self) {
        self.usart.uartier().modify(|_, w| w.etbei().set_bit());
    }

    pub fn clear_tx_idle_interrupt(&self) {
        self.usart.uartier().modify(|_, w| w.etbei().clear_bit());
    }

    pub fn set_rx_data_available_interrupt(&self) {
        self.usart.uartier().modify(|_, w| w.erbfi().set_bit());
    }

    pub fn clear_rx_data_available_interrupt(&self) {
        self.usart.uartier().modify(|_, w| w.erbfi().clear_bit());
    }
}
