// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]
#![allow(dead_code)]

use app_test_usb::{handle, signals};
use util_error::{ErrorCode, KERNEL_ERROR_UNKNOWN};
use userspace::time::Instant;
use userspace::{entry, syscall};

use aligned::{Aligned, A4};
use hal_usb::driver::UsbDriver;
use hal_usb::{Direction, StringDescriptorRef};

use usb_driver::{EpIn, EpOut, UsbConfig};
use usb_stack::{
    DescriptorSource,
    UsbAction,
    UsbClass,
};
use protocol_usb_cdc_acm::CdcAcm;

const USB_VENDOR_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(1);
const USB_PRODUCT_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(2);
const USB_SERIAL_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(3);
const USB_CDC_COMM_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(4);
const USB_CDC_DATA_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(5);

const USB_CLASS_CDC: u8 = 0x02;
const USB_CLASS_CDC_DATA: u8 = 0x0a;
const CDC_SUBCLASS_ACM: u8 = 0x02;
const CDC_PROTOCOL_NONE: u8 = 0x00;

const CS_INTERFACE: u8 = 0x24;
const CDC_TYPE_HEADER: u8 = 0x00;
const CDC_TYPE_ACM: u8 = 0x02;
const CDC_TYPE_UNION: u8 = 0x06;

static DEVICE_DESC: hal_usb::DeviceDescriptor = hal_usb::DeviceDescriptor {
    device_class: hal_usb::DeviceClass::SPECIFIED_BY_INTERFACE,
    device_sub_class: 0x00,
    device_protocol: 0x00,
    max_packet_size: 64,
    vendor_id: 0x18d1,
    product_id: 0x023b,
    device_release_num: 0x0100,
    manufacturer: USB_VENDOR_HANDLE,
    product: USB_PRODUCT_HANDLE,
    serial_num: USB_SERIAL_HANDLE,
};
const CONFIG_DESC: hal_usb::ConfigDescriptor = hal_usb::ConfigDescriptor {
    configuration_value: 1,
    max_power: 250,
    self_powered: false,
    remote_wakeup: false,
    interfaces: &[
        hal_usb::InterfaceDescriptor {
            name: USB_CDC_COMM_HANDLE,
            interface_number: 0,
            alternate_setting: 0,
            interface_class: USB_CLASS_CDC,
            interface_sub_class: CDC_SUBCLASS_ACM,
            interface_protocol: CDC_PROTOCOL_NONE,
            func_descs: &[
                hal_usb::FunctionalDescriptor::raw(CS_INTERFACE, &[CDC_TYPE_HEADER, 0x10, 0x01]),
                hal_usb::FunctionalDescriptor::raw(CS_INTERFACE, &[CDC_TYPE_ACM, 0x02]),
                hal_usb::FunctionalDescriptor::raw(
                    CS_INTERFACE,
                    &[
                        CDC_TYPE_UNION,
                        0, // comm_if
                        1, // data_if
                    ],
                ),
            ],
            endpoints: &[hal_usb::EndpointDescriptor {
                direction: Direction::DeviceToHost,
                endpoint_num: 1,
                interval: 255,
                max_packet_size: 8,
                transfer_type: hal_usb::TransferType::Interrupt,
            }],
        },
        hal_usb::InterfaceDescriptor {
            name: USB_CDC_DATA_HANDLE,
            interface_number: 1,
            alternate_setting: 0,
            interface_class: USB_CLASS_CDC_DATA,
            interface_sub_class: 0,
            interface_protocol: CDC_PROTOCOL_NONE,
            func_descs: &[],
            endpoints: &[
                hal_usb::EndpointDescriptor {
                    direction: Direction::HostToDevice,
                    endpoint_num: 2,
                    interval: 0,
                    max_packet_size: 64,
                    transfer_type: hal_usb::TransferType::Bulk,
                },
                hal_usb::EndpointDescriptor {
                    direction: Direction::DeviceToHost,
                    endpoint_num: 3,
                    interval: 0,
                    max_packet_size: 64,
                    transfer_type: hal_usb::TransferType::Bulk,
                },
            ],
        },
    ],
};

const STRING_DESC_0: hal_usb::StringDescriptor0 = hal_usb::StringDescriptor0 {
    langs: &[
        // English - United States
        0x0409,
    ],
};

const VENDOR_ID: hal_usb::StringDescriptorRef = hal_usb::string_descriptor!("Google Inc.").as_ref();
const PRODUCT_ID_DEFAULT: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("Earlgrey").as_ref();
const USB_COMM: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("CDC Comm Interface").as_ref();
const USB_DATA: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("CDC Data Interface").as_ref();

struct MyDescriptors<'a> {
    serial_desc_bytes: StringDescriptorRef<'a>,
    product_desc_bytes: StringDescriptorRef<'a>,
}

impl DescriptorSource for MyDescriptors<'_> {
    const DEVICE_DESC_BYTES: &'static Aligned<A4, [u8]> = &Aligned(DEVICE_DESC.serialize());
    const CONFIG_DESC_BYTES: &'static Aligned<A4, [u8]> =
        &Aligned(CONFIG_DESC.serialize::<{ CONFIG_DESC.total_size() }>());
    const STRING_DESC_0_BYTES: &'static Aligned<A4, [u8]> =
        &Aligned(STRING_DESC_0.serialize::<{ STRING_DESC_0.total_size() }>());
    const DEVICE_STATUS: Aligned<A4, [u8; 2]> = Aligned([1u8, 0]);

    fn get_string(
        &self,
        handle: hal_usb::StringHandle,
        _lang: u16,
    ) -> Option<hal_usb::StringDescriptorRef<'_>> {
        match handle {
            USB_VENDOR_HANDLE => Some(VENDOR_ID),
            USB_PRODUCT_HANDLE => Some(self.product_desc_bytes),
            USB_SERIAL_HANDLE => Some(self.serial_desc_bytes),
            USB_CDC_COMM_HANDLE => Some(USB_COMM),
            USB_CDC_DATA_HANDLE => Some(USB_DATA),
            _ => None,
        }
    }
}

fn handle_usb() -> Result<(), ErrorCode> {
    let mut serial_num_buffer = Aligned::<A4, _>([0_u8; 130]);
    let descriptors = MyDescriptors {
        serial_desc_bytes: hal_usb::hex_utf16_descriptor_aligned(&mut serial_num_buffer, b"12345")
            .unwrap(),
        product_desc_bytes: PRODUCT_ID_DEFAULT,
    };
    const USB_EP_ACM_INT_IN: EpIn = EpIn {
        num: 1,
        buf_pool_size: 1,
    };
    const USB_EP_ACM_OUT: EpOut = EpOut {
        num: 2,
        set_nak: false,
    };
    const USB_EP_ACM_IN: EpIn = EpIn {
        num: 3,
        buf_pool_size: 1,
    };

    const USB_CONFIG: UsbConfig =
        UsbConfig::new(&[USB_EP_ACM_INT_IN, USB_EP_ACM_IN], &[USB_EP_ACM_OUT]);
    let mut usb = usb_driver::Usb::new(unsafe { usbdev::Usbdev::new() }, USB_CONFIG);
    let mut ep0 = usb_stack::SimpleEp0::new();
    let mut cdc_acm = CdcAcm::new(0, 1, 2, 3);

    loop {
        let wait_return = syscall::object_wait(
            handle::USBDEV_INTERRUPTS,
            signals::USBDEV_PKT_RECEIVED
                | signals::USBDEV_PKT_SENT
                | signals::USBDEV_DISCONNECTED
                | signals::USBDEV_HOST_LOST
                | signals::USBDEV_LINK_RESET
                | signals::USBDEV_LINK_SUSPEND
                | signals::USBDEV_LINK_RESUME
                | signals::USBDEV_AV_OUT_EMPTY
                | signals::USBDEV_RX_FULL
                | signals::USBDEV_AV_OVERFLOW
                | signals::USBDEV_RX_CRC_ERR
                | signals::USBDEV_RX_PID_ERR
                | signals::USBDEV_RX_BITSTUFF_ERR
                | signals::USBDEV_FRAME
                | signals::USBDEV_AV_SETUP_EMPTY,
            Instant::MAX,
        )?;

        if wait_return.user_data != 0 {
            pw_log::error!("Incorrect WaitReturn values");
            return Err(KERNEL_ERROR_UNKNOWN);
        }

        while let Some(event) = usb.poll() {
            let mut action = match cdc_acm.handle_event(event) {
                Ok(a) => a,
                Err(e) => ep0.handle_event(e, &descriptors).unwrap_or(UsbAction::None),
            };
            action.run(&mut usb);
        }
    }
}

fn usb_setup_pinmux() {
    use top_earlgrey::{PinmuxInsel, PinmuxPeripheralIn};
    let mut pinmux = unsafe { pinmux::PinmuxAon::new() };

    pinmux
        .regs_mut()
        .mio_periph_insel()
        .at(PinmuxPeripheralIn::UsbdevSense as usize)
        .modify(|_| (PinmuxInsel::ConstantOne as u32).into());
}

#[entry]
fn entry() -> ! {
    usb_setup_pinmux();
    let ret = match handle_usb() {
        Ok(()) => Ok(()),
        Err(e) => {
            pw_log::error!("Error {:x}", e.0.get());
            Err(pw_status::Error::Unknown)
        }
    };
    let _ = syscall::debug_shutdown(ret);
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in {}", module_path!() as &str);
    loop {}
}
