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
use hal_usb::{
    ConfigDescriptor, DeviceDescriptor, EndpointDescriptor, FunctionalDescriptor,
    InterfaceDescriptor, StringDescriptorRef,
};

use usb_driver::UsbConfig;
use usb_stack::{DescriptorSource, UsbAction, UsbClass};

use protocol_usb_cdc_acm::{CdcAcm, CdcAcmBuilder};

const USB_VENDOR_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(1);
const USB_PRODUCT_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(2);
const USB_SERIAL_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(3);
const USB_CDC_COMM_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(4);
const USB_CDC_DATA_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(5);

const CDC_BUILDER: CdcAcmBuilder = CdcAcmBuilder::new(
    0, // comm_if: Communication Interface index
    1, // data_if: Data Interface index
    1, // comm_ep: Communication IN endpoint (Interrupt)
    2, // data_out_ep: Data OUT endpoint (Bulk)
    3, // data_in_ep: Data IN endpoint (Bulk)
);

static DEVICE_DESC: DeviceDescriptor = DeviceDescriptor {
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

// Fragmented Static Assembly
static CDC_COMM_FUNC_DESCS: [FunctionalDescriptor; 3] = CDC_BUILDER.comm_func_descs();
static CDC_COMM_ENDPOINTS: [EndpointDescriptor; 1] = CDC_BUILDER.comm_endpoints();
static CDC_DATA_ENDPOINTS: [EndpointDescriptor; 2] = CDC_BUILDER.data_endpoints();

const CDC_INTERFACES: [InterfaceDescriptor; 2] = [
    CDC_BUILDER.comm_interface(
        USB_CDC_COMM_HANDLE,
        &CDC_COMM_FUNC_DESCS,
        &CDC_COMM_ENDPOINTS,
    ),
    CDC_BUILDER.data_interface(USB_CDC_DATA_HANDLE, &CDC_DATA_ENDPOINTS),
];

const CONFIG_DESC: ConfigDescriptor = ConfigDescriptor {
    configuration_value: 1,
    max_power: 250,
    self_powered: false,
    remote_wakeup: false,
    interfaces: &CDC_INTERFACES,
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

    const CDC_EPS: (
        [protocol_usb_cdc_acm::EpIn; 2],
        [protocol_usb_cdc_acm::EpOut; 1],
    ) = CDC_BUILDER.eps();
    const USB_CONFIG: UsbConfig = UsbConfig::new(&CDC_EPS.0, &CDC_EPS.1);

    let mut usb = usb_driver::Usb::new(unsafe { usbdev::Usbdev::new() }, USB_CONFIG);
    let mut ep0 = usb_stack::SimpleEp0::new();
    let mut cdc_acm = CdcAcm::<1024, 1024>::new(CDC_BUILDER);

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

        // Loopback received data to send buffer
        while let Some(byte) = cdc_acm.rx_queue.pop() {
            let _ = cdc_acm.tx_queue.push(byte);
        }

        // Initiate any pending transmissions
        cdc_acm.poll_transmit(&mut usb);
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
