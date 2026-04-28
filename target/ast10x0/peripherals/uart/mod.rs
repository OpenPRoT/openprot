// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use ast1060_pac as device;
use bitflags::bitflags;
use core::cell::UnsafeCell;
use core::marker::PhantomData;
use embedded_hal_nb::serial as serial_nb;
use embedded_io::{Read, Write};

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Error {
	Frame,
	Parity,
	Noise,
	BufFull,
}

#[derive(Debug)]
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

/// Receiver FIFO Interrupt trigger level
#[derive(Debug)]
#[repr(u8)]
pub enum FifoTriggerLevel {
	// 1 byte trigger level
	OneByte = 0b00,
	// 4 byte trigger level
	FourByte = 0b01,
	// 8 byte trigger level
	EightByte = 0b10,
	// 14 byte trigger level
	FourteenByte = 0b11,
}

bitflags! {
	#[derive(Debug)]
	pub struct LineStatus: u8 {
		/// here is at least one parity error, framing error, or break indication in the FIFO.
		///
		/// This bit is only active when FIFOs are enabled.
		/// This bit is cleared when the `UART_LSR` is read.
		const ErrorInReceiverFifo = 0x80;
		/// Transmitter empty
		///
		/// When FIFO enabled, the Transmitter Shift Register and FIFO are both empty.
		/// When FIFO disabled, the Transmitter Shift Register and UART_THR are both empty.
		const TransmitterEmpty = 0x40;
		/// Transmitter Holding Register Empty
		///
		/// This bit is set whenever data is transferred from UART_THR
		/// or TX FIFO to the transmitter shift register
		/// and no new data has been written to the UART THR or TX FIFO.
		/// This also causes a THRE Interrupt to occur, if THRE Interrupt is enabled
		const TransmitterHoldingRegisterEmpty = 0x20;
		/// Break interrupt
		///
		/// The serial input is held in a logic `0` state for longer
		/// than the sum of start time + data bits + parity + stop bits.
		/// A break condition on serial input causes one and only one character,
		/// consisting of all zeros, to be received by the UART
		const BreakInterrupt = 0x10;
		/// Framing error
		///
		/// A received character did not have a valid stop bit.
		const FramingError = 0x08;
		/// Parity error
		///
		/// Receive parity error while parity enable was set.
		const ParityError = 0x04;
		/// Overrun error
		///
		/// Character was received while the receiver or FIFO was full.
		const OverrunError = 0x02;
		/// Data ready
		///
		/// The receiver contains at least one character.
		const DataReady = 0x01;
	}
}
pub struct Usart {
	usart: *const device::uart::RegisterBlock,
	_not_sync: PhantomData<UnsafeCell<()>>, // makes Usart !Sync
}

impl embedded_io::ErrorType for Usart {
	type Error = Error;
}

impl embedded_io::Error for Error {
	fn kind(&self) -> embedded_io::ErrorKind {
		embedded_io::ErrorKind::Other
	}
}

impl serial_nb::Error for Error {
	fn kind(&self) -> serial_nb::ErrorKind {
		match self {
			Error::Frame => serial_nb::ErrorKind::FrameFormat,
			Error::Parity => serial_nb::ErrorKind::Parity,
			Error::Noise => serial_nb::ErrorKind::Noise,
			Error::BufFull => serial_nb::ErrorKind::Overrun,
		}
	}
}

impl serial_nb::ErrorType for Usart {
	type Error = Error;
}

impl Write for Usart {
	fn flush(&mut self) -> Result<(), Error> {
		while !self.is_tx_idle() {}
		Ok(())
	}

	fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
		for (n, byte) in buf.iter().enumerate() {
			if !self.is_tx_full() {
				// This is unsafe because we can transmit 7, 8 or 9 bits but the
				// interface can't know what it's been configured for.
				self.regs()
					.uartthr()
					.write(|w| unsafe { w.bits(*byte as u32) });
			} else {
				if n == 0 {
					// spec demands to block until atleast one byte has been written
					continue;
				}
				return Ok(n);
			}
		}
		Ok(buf.len())
	}
}

impl Read for Usart {
	fn read(&mut self, out: &mut [u8]) -> Result<usize, Self::Error> {
		if out.is_empty() {
			return Ok(0);
		}

		let mut count = 0;
		// Block until at least one byte is available, then drain what is immediately readable.
		while count == 0 {
			match self.try_read_byte() {
				Ok(byte) => {
					out[count] = byte;
					count += 1;
				}
				Err(nb::Error::WouldBlock) => continue,
				Err(nb::Error::Other(e)) => return Err(e),
			}
		}

		while count < out.len() {
			match self.try_read_byte() {
				Ok(byte) => {
					out[count] = byte;
					count += 1;
				}
				Err(nb::Error::WouldBlock) => break,
				Err(nb::Error::Other(e)) => return Err(e),
			}
		}

		Ok(count)
	}
}

impl serial_nb::Write<u8> for Usart {
	fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
		if !self.is_tx_full() {
			// This is unsafe because we can transmit 7, 8 or 9 bits but the
			// interface can't know what it's been configured for.
			self.regs()
				.uartthr()
				.write(|w| unsafe { w.bits(word as u32) });
			Ok(())
		} else {
			Err(nb::Error::WouldBlock)
		}
	}

	fn flush(&mut self) -> nb::Result<(), Self::Error> {
		if self.is_tx_idle() {
			Ok(())
		} else {
			Err(nb::Error::WouldBlock)
		}
	}
}

impl serial_nb::Read<u8> for Usart {
	fn read(&mut self) -> nb::Result<u8, Self::Error> {
		self.try_read_byte()
	}
}
pub enum Rate {
	Baud9600,
	Baud19200,
	MBaud1_5,
}

impl Usart {
	#[inline]
	fn try_read_byte(&self) -> nb::Result<u8, Error> {
		// Read LSR exactly once per byte so error bits and DR correspond to the same FIFO state.
		let lsr = self.regs().uartlsr().read();
		if !lsr.dr().bit() {
			return Err(nb::Error::WouldBlock);
		}

		let byte = self.regs().uartrbr().read().bits() as u8;
		if lsr.fe().bit_is_set() {
			Err(nb::Error::Other(Error::Frame))
		} else if lsr.pe().bit_is_set() {
			Err(nb::Error::Other(Error::Parity))
		} else if self.is_rx_noise_err() {
			Err(nb::Error::Other(Error::Noise))
		} else {
			Ok(byte)
		}
	}

	/// Create a new USART instance from a raw register-block pointer.
	///
	/// Configures RX/TX FIFO, 8 byte RX trigger level, 1.5MBaud, 8n1, and
	/// enables all interrupts.
	///
	/// # Safety
	///
	/// - `usart` must be a valid, non-null pointer to the AST1060 UART register block.
	/// - The pointed register block must remain valid for the lifetime of this `Usart`.
	/// - Caller must enforce global ownership/coordination so concurrent mutable access
	///   does not occur through other code paths.
	pub unsafe fn new(usart: *const device::uart::RegisterBlock) -> Self {
		let this = Self {
			usart,
			_not_sync: PhantomData,
		};

		unsafe {
			this.regs().uartfcr().write(|w| {
				w.enbl_uartfifo().set_bit();
				w.rx_fiforst().set_bit();
				w.tx_fiforst().set_bit();
				w.define_the_rxr_fifointtrigger_level().bits(0b10)
			});
		}

		this
			.set_rate(Rate::MBaud1_5)
			.set_8n1()
			.interrupt_enable()
	}

	#[inline]
	fn regs(&self) -> &device::uart::RegisterBlock {
		// SAFETY: `Usart` construction is `unsafe`, so caller upholds pointer validity,
		// non-nullness, and aliasing/ownership requirements.
		unsafe { &*self.usart }
	}

	/// Set the baud rate
	///
	/// These baud rates assume that the uart clock is set to 24Mhz.
	pub fn set_rate(self, rate: Rate) -> Self {
		// These baud rates assume that the uart clock is set to 24Mhz.

		// Enable DLAB to access divisor latch registers
		self.regs().uartlcr().modify(|_, w| w.dlab().set_bit());

		// Divisor = 24M / (13 * 16 * Baud Rate)
		match rate {
			Rate::Baud9600 => {
				self.regs().uartdlh().write(|w| unsafe { w.bits(0) });
				self.regs().uartdll().write(|w| unsafe { w.bits(12) });
			}
			Rate::Baud19200 => {
				self.regs().uartdlh().write(|w| unsafe { w.bits(0) });
				self.regs().uartdll().write(|w| unsafe { w.bits(6) });
			}
			Rate::MBaud1_5 => {
				self.regs().uartdlh().write(|w| unsafe { w.bits(0) });
				self.regs().uartdll().write(|w| unsafe { w.bits(1) });
			}
		}
		// Disable DLAB to access other registers
		self.regs().uartlcr().modify(|_, w| w.dlab().clear_bit());

		self
	}

	/// Enable all interrupts
	///
	/// - Modem Status Interrupt
	/// - Receiver Line Status Interrupt
	/// - Transmitter Holding Register Empty Interrupt
	/// - Received Data Available Interrupt
	pub fn interrupt_enable(self) -> Self {
		self.regs().uartier().write(|w| {
			w.erbfi().set_bit(); // Enable Received Data Available Interrupt
			w.etbei().set_bit(); // Enable Transmitter Holding Register Empty Interrupt
			w.elsi().set_bit(); // Enable Receiver Line Status Interrupt
			w.edssi().set_bit(); // Enable Modem Status Interrupt
			w
		});

		self
	}

	/// Set the Receiver FIFO Interrupt trigger level
	pub fn set_rx_fifo_trigger_level(&self, level: FifoTriggerLevel) {
		unsafe {
			self.regs().uartfcr().modify(|_, w| {
				w.define_the_rxr_fifointtrigger_level().bits(level as u8)
			});
		}
	}

	pub fn set_8n1(self) -> Self {
		self
	}

	pub fn is_tx_full(&self) -> bool {
		!self.regs().uartlsr().read().thre().bit()
	}

	pub fn is_rx_empty(&self) -> bool {
		!self.regs().uartlsr().read().dr().bit()
	}

	pub fn is_rx_frame_err(&self) -> bool {
		self.regs().uartlsr().read().fe().bit_is_set()
	}

	pub fn is_rx_parity_err(&self) -> bool {
		self.regs().uartlsr().read().pe().bit_is_set()
	}

	pub fn is_rx_noise_err(&self) -> bool {
		// self.usart.uartlsr().read().rxnoise().bit()
		false
	}

	pub fn read_interrupt_status(&self) -> InterruptDecoding {
		InterruptDecoding::try_from(
			self.regs().uartiir().read().intdecoding_table().bits() & 0x07,
		)
		.unwrap_or(InterruptDecoding::Unknown)
	}

	pub fn read_line_status(&self) -> LineStatus {
		let status = self.regs().uartlsr().read().bits() as u8;
		LineStatus::from_bits_truncate(status)
	}

	/// Non-blocking drain of the RX FIFO into `out`.
	///
	/// Copies as many bytes as are immediately available (up to `out.len()`)
	/// and returns the count.  Returns `0` if the FIFO is empty; the caller
	/// should arm the `RX_DATA_AVAILABLE` interrupt and retry.
	pub fn try_read_available(&self, out: &mut [u8]) -> usize {
		let mut count = 0;
		while count < out.len() {
			match self.try_read_byte() {
				Ok(byte) => {
					out[count] = byte;
					count += 1;
				}
				Err(nb::Error::WouldBlock) => break,
				Err(nb::Error::Other(_)) => break,
			}
		}
		count
	}

	pub fn read_modem_status(&self) -> u8 {
		self.regs().uartmsr().read().bits() as u8
	}

	pub fn is_tx_idle(&self) -> bool {
		self.regs().uartlsr().read().txter_empty().bit_is_set()
	}

	/// Enables the TX idle interrupt (ETBEI)
	pub fn set_tx_idle_interrupt(&self) {
		self.regs().uartier().modify(|_, w| w.etbei().set_bit());
	}

	/// Disables the TX idle interrupt (ETBEI)
	pub fn clear_tx_idle_interrupt(&self) {
		// self.regs().uartier().write(|w| w.etbei().clear_bit());
		self.regs().uartier().modify(|_, w| w.etbei().clear_bit());
	}

	/// Enables the RX idle interrupt (ERBFI)
	pub fn set_rx_data_available_interrupt(&self) {
		self.regs().uartier().modify(|_, w| w.erbfi().set_bit());
	}

	/// Disables the RX idle interrupt (ERBFI)
	pub fn clear_rx_data_available_interrupt(&self) {
		self.regs().uartier().modify(|_, w| w.erbfi().clear_bit());
	}
}
