// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]
#![allow(dead_code)]

use pw_status::Error;
use test_usb_dfu_codegen::{handle, signals};
use userspace::time::Instant;
use userspace::{entry, syscall};

use aligned::{Aligned, A4};
use hal_usb::{ConfigDescriptor, DeviceDescriptor, StringDescriptorRef};

use hal_usb::driver::UsbDriver;
use usb_driver::UsbConfig;
use usb_stack::{DescriptorSource, UsbAction, UsbClass};

use protocol_usb_dfu::{DfuBuilder, DfuClass, DfuHandler, DfuResult, DfuStatus};

const USB_VENDOR_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(1);
const USB_PRODUCT_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(2);
const USB_SERIAL_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(3);
const USB_DFU_INTERFACE_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(4);

const DFU_BUILDER: DfuBuilder = DfuBuilder::new(
    0,    // interface_num
    1,    // alt_settings (1 for now)
    2048, // transfer_size
);

const DEVICE_DESC: DeviceDescriptor = DeviceDescriptor {
    device_class: hal_usb::DeviceClass::SPECIFIED_BY_INTERFACE,
    device_sub_class: 0x00,
    device_protocol: 0x00,
    max_packet_size: 64,
    vendor_id: 0x18d1,  // Google, Inc.
    product_id: 0x503a, // STWG USB Fullspeed IP.
    device_release_num: 0x0100,
    manufacturer: USB_VENDOR_HANDLE,
    product: USB_PRODUCT_HANDLE,
    serial_num: USB_SERIAL_HANDLE,
};

const CONFIG_DESC: ConfigDescriptor = ConfigDescriptor {
    configuration_value: 1,
    max_power: 250,
    self_powered: false,
    remote_wakeup: false,
    interfaces: &[DFU_BUILDER.interface(
        0,
        USB_DFU_INTERFACE_HANDLE,
        &[DFU_BUILDER.functional_descriptor()],
    )],
};

const STRING_DESC_0: hal_usb::StringDescriptor0 = hal_usb::StringDescriptor0 {
    langs: &[
        // English - United States
        0x0409,
    ],
};

const VENDOR_ID: hal_usb::StringDescriptorRef = hal_usb::string_descriptor!("Google Inc.").as_ref();
const PRODUCT_ID_DEFAULT: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("Earlgrey DFU").as_ref();
const USB_DFU: hal_usb::StringDescriptorRef = hal_usb::string_descriptor!("DFU Interface").as_ref();

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
            USB_DFU_INTERFACE_HANDLE => Some(USB_DFU),
            _ => None,
        }
    }
}

struct MyDfuHandler;

impl DfuHandler for MyDfuHandler {
    fn dnload(&mut self, alt: u8, block_num: u16, data: &[u8]) -> DfuResult {
        pw_log::info!(
            "DNLOAD: alt={}, block={}, len={}",
            alt,
            block_num,
            data.len()
        );
        DfuResult::Ok
    }

    fn upload(&mut self, alt: u8, block_num: u16, data: &mut [u8]) -> Result<usize, DfuStatus> {
        pw_log::info!(
            "UPLOAD: alt={}, block={}, len={}",
            alt,
            block_num,
            data.len()
        );
        // Send a 2K block with a fixed pattern
        for (i, byte) in data.iter_mut().enumerate() {
            *byte = (i & 0xFF) as u8;
        }
        Ok(data.len())
    }

    fn manifest(&mut self) -> DfuResult {
        pw_log::info!("MANIFEST");
        DfuResult::Ok
    }

    fn abort(&mut self) {
        pw_log::info!("ABORT");
    }
}

fn handle_usb() -> Result<(), Error> {
    let mut serial_num_buffer = Aligned::<A4, _>([0_u8; 130]);
    let descriptors = MyDescriptors {
        serial_desc_bytes: hal_usb::hex_utf16_descriptor_aligned(
            &mut serial_num_buffer,
            b"DFU-12345",
        )
        .unwrap(),
        product_desc_bytes: PRODUCT_ID_DEFAULT,
    };

    const USB_CONFIG: UsbConfig = UsbConfig::new(&[], &[]);

    let mut usb = usb_driver::Usb::new(unsafe { usbdev::Usbdev::new() }, USB_CONFIG);
    let mut ep0 = usb_stack::SimpleEp0::new();
    let mut dfu = DfuClass::<_, 2048>::new(DFU_BUILDER, MyDfuHandler);

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
            return Err(Error::Unknown);
        }

        while let Some(event) = usb.poll() {
            let mut action = match dfu.handle_event(event) {
                Ok(a) => a,
                Err(e) => ep0.handle_event(e, &descriptors).unwrap_or(UsbAction::None),
            };
            action.run(&mut usb);
        }

        // Initiate any pending transmissions (e.g. UPLOAD blocks)
        dfu.poll(&mut usb);
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
    let ret = handle_usb();
    let _ = syscall::debug_shutdown(ret);
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in {}", module_path!() as &str);
    loop {}
}
