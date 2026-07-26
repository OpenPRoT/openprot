// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use earlgrey_gpio::GpioPin;
use earlgrey_pinmux::Pad;
use top_earlgrey::{PinmuxOutsel as Outsel, PinmuxPeripheralIn as PeriphIn};

use crate::config::*;
use crate::Pinout;

type PC = PinoutConfig;

pub struct DualSideBySide;
impl DualSideBySide {
    // Outputs use Gpio [0..15].
    pub const RST_CTRL0_N: GpioPin = GpioPin::Pin0;
    pub const RST_CTRL1_N: GpioPin = GpioPin::Pin1;
    pub const SPI_RESET_N: GpioPin = GpioPin::Pin2;
    pub const SPI_MUX_EN_N: GpioPin = GpioPin::Pin3;
    pub const SPI_MUX_CTRL: GpioPin = GpioPin::Pin4;
    pub const SPI_HOST0_WP_N: GpioPin = GpioPin::Pin5;
    pub const SPI_HOST1_WP_N: GpioPin = GpioPin::Pin6;
    pub const USB_MUX_CTRL: GpioPin = GpioPin::Pin7;
    pub const EXT_DEBUG_N: GpioPin = GpioPin::Pin8;

    // Inputs use Gpio [16..31].
    pub const USB_PRESENCE_N: GpioPin = GpioPin::Pin16;
    pub const RST_MON0_N: GpioPin = GpioPin::Pin17;
    pub const RST_MON1_N: GpioPin = GpioPin::Pin18;
    pub const SW_STRAP0: GpioPin = GpioPin::Pin22;
    pub const SW_STRAP1: GpioPin = GpioPin::Pin23;
    pub const SW_STRAP2: GpioPin = GpioPin::Pin24;
}

impl Pinout for DualSideBySide {
    #[rustfmt::skip]
    const PINOUT: &[PinoutConfig] = &[
        PC::func_in( "UART0_RX",       "IOC3",  PeriphIn::Uart0Rx,         Pad::IOC3, IN_PULL_UP),
        PC::func_out("UART0_TX",       "IOC4",  Outsel::Uart0Tx,           Pad::IOC4, OUT_PULL_UP),
        PC::func_in( "USBDEV_SENSE",   "none",  PeriphIn::UsbdevSense,     Pad::ConstantOne, IN_PULL_NONE),
        PC::gpio_in( "USB_PRESENCE_N", "IOR11", Self::USB_PRESENCE_N,      Pad::IOR11, IN_PULL_UP),
        PC::gpio_out("USB_MUX_CTRL",   "IOC6",  Self::USB_MUX_CTRL,        Pad::IOC6, OUT_PUSH_PULL),
        PC::gpio_out("RST_CTRL0_N",    "IOA0",  Self::RST_CTRL0_N,         Pad::IOA0, OUT_PUSH_PULL),
        PC::gpio_out("RST_CTRL1_N",    "IOA1",  Self::RST_CTRL1_N,         Pad::IOA1, OUT_PUSH_PULL),
        PC::gpio_in( "RST_MON0_N",     "IOA2",  Self::RST_MON0_N,          Pad::IOA2, IN_PULL_NONE),
        PC::gpio_in( "RST_MON1_N",     "IOA5",  Self::RST_MON1_N,          Pad::IOA5, IN_PULL_NONE),
        PC::gpio_out("SPI_RESET_N",    "IOA7",  Self::SPI_RESET_N,         Pad::IOA7, OUT_PULL_UP),
        PC::func_in( "SPI_DEV_CS1_L",  "IOA4",  PeriphIn::SpiDeviceTpmCsb, Pad::IOA4, IN_PULL_UP),
        PC::gpio_out("SPI_MUX_EN_N",   "IOB7",  Self::SPI_MUX_EN_N,        Pad::IOB7, OUT_PULL_UP),
        PC::gpio_out("SPI_MUX_CTRL",   "IOB8",  Self::SPI_MUX_CTRL,        Pad::IOB8, OUT_PULL_UP),
        PC::gpio_out("SPI_HOST0_WP_N", "IOA3",  Self::SPI_HOST0_WP_N,      Pad::IOA3, OUT_PULL_UP),
        PC::gpio_out("SPI_HOST1_WP_N", "IOA6",  Self::SPI_HOST1_WP_N,      Pad::IOA6, OUT_PULL_UP),
        PC::gpio_out("EXT_DEBUG_N",    "IOC9",  Self::EXT_DEBUG_N,         Pad::IOC9, OUT_PULL_UP),
        PC::func_out("SPI_HOST1_CLK",  "IOB0",  Outsel::SpiHost1Sck,       Pad::IOB0, OUT_PULL_UP),
        PC::func_out("SPI_HOST1_CS_L", "IOB3",  Outsel::SpiHost1Csb,       Pad::IOB3, OUT_PULL_UP),
        PC::func_io( "SPI_HOST1_D0",   "IOB1",  PeriphIn::SpiHost1Sd0,     Pad::IOB1, IN_PULL_UP),
        PC::func_io( "SPI_HOST1_D1",   "IOB2",  PeriphIn::SpiHost1Sd1,     Pad::IOB2, IN_PULL_UP),
        PC::gpio_in( "SW_STRAP2",      "IOC2",  Self::SW_STRAP2,           Pad::IOC2, IN_PULL_NONE),
        PC::gpio_in( "SW_STRAP1",      "IOC1",  Self::SW_STRAP1,           Pad::IOC1, IN_PULL_NONE),
        PC::gpio_in( "SW_STRAP0",      "IOC0",  Self::SW_STRAP0,           Pad::IOC0, IN_PULL_NONE),
    ];
}
