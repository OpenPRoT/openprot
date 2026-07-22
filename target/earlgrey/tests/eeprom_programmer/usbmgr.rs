// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use aligned::{Aligned, A4};
use hal_flash::{Flash, FlashAddress};
use hal_usb::driver::UsbDriver;
use hal_usb::{ConfigDescriptor, DeviceDescriptor, StringDescriptorRef};
use lc_ctrl::LcCtrl;
use pinmux::PinmuxAon;
use protocol_usb_cdc_acm::{CdcAcm, CdcAcmBuilder};
use protocol_usb_dfu::{DfuBuilder, DfuClass, DfuHandler, DfuStatus};
use pw_status::Error;
use services_flash_client::FlashIpcClient;
use usb_driver::UsbConfig;
use usb_stack::{DescriptorSource, UsbAction, UsbClass};
use usbdev::Usbdev;
use usbmgr_codegen::{handle, signals};
use userspace::time::Instant;
use userspace::{process_entry, syscall};
use util_error::ErrorCode;
use util_ipc::IpcHandle;
use zerocopy::IntoBytes;

const USB_VENDOR_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(1);
const USB_PRODUCT_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(2);
const USB_SERIAL_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(3);
const USB_CDC_COMM_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(4);
const USB_CDC_DATA_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(5);
const DFU_FIRMWARE_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(6);
const DFU_UDS_CERT_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(7);
const DFU_CDI0_CERT_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(8);
const DFU_CDI1_CERT_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(9);
const DFU_SPI_EEPROM_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(10);

const DFU_ALT_FIRMWARE: u8 = 0;
const DFU_ALT_UDS_CERT: u8 = 1;
const DFU_ALT_CDI0_CERT: u8 = 2;
const DFU_ALT_CDI1_CERT: u8 = 3;
const DFU_ALT_SPI_EEPROM0: u8 = 4;

const DFU_BUILDER: DfuBuilder = DfuBuilder::new(
    2,    // interface_num
    5,    // alt_settings
    2048, // transfer_size
);

const CDC_BUILDER: CdcAcmBuilder = CdcAcmBuilder::new(
    0, // comm_if
    1, // data_if
    1, // comm_ep
    2, // data_out_ep
    3, // data_in_ep
);

const DEVICE_DESC: DeviceDescriptor = DeviceDescriptor {
    device_class: hal_usb::DeviceClass::SPECIFIED_BY_INTERFACE,
    device_sub_class: 0x00,
    device_protocol: 0x00,
    max_packet_size: 64,
    vendor_id: 0x18d1,
    product_id: 0x503a,
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
    interfaces: &[
        CDC_BUILDER.comm_interface(
            USB_CDC_COMM_HANDLE,
            &CDC_BUILDER.comm_func_descs(),
            &CDC_BUILDER.comm_endpoints(),
        ),
        CDC_BUILDER.data_interface(USB_CDC_DATA_HANDLE, &CDC_BUILDER.data_endpoints()),
        DFU_BUILDER.interface(DFU_ALT_FIRMWARE, DFU_FIRMWARE_HANDLE, &[]),
        DFU_BUILDER.interface(DFU_ALT_UDS_CERT, DFU_UDS_CERT_HANDLE, &[]),
        DFU_BUILDER.interface(DFU_ALT_CDI0_CERT, DFU_CDI0_CERT_HANDLE, &[]),
        DFU_BUILDER.interface(DFU_ALT_CDI1_CERT, DFU_CDI1_CERT_HANDLE, &[]),
        DFU_BUILDER.interface(
            DFU_ALT_SPI_EEPROM0,
            DFU_SPI_EEPROM_HANDLE,
            &[DFU_BUILDER.functional_descriptor()],
        ),
    ],
};

const STRING_DESC_0: hal_usb::StringDescriptor0 = hal_usb::StringDescriptor0 { langs: &[0x0409] };

const VENDOR_ID: hal_usb::StringDescriptorRef = hal_usb::string_descriptor!("Google Inc.").as_ref();
const PRODUCT_ID_DEFAULT: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("OpenPRoT Earlgrey EEPROM Programmer").as_ref();
const USB_COMM: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("CDC Comm Interface").as_ref();
const USB_DATA: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("CDC Data Interface").as_ref();
const DFU_FIRMWARE: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("Application / Transport Firmware").as_ref();
const DFU_UDS_CERT: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("UDS Certificate").as_ref();
const DFU_CDI0_CERT: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("CDI_0 Certificate").as_ref();
const DFU_CDI1_CERT: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("CDI_1 Certificate").as_ref();
const DFU_SPI_EEPROM: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("SPI EEPROM 0").as_ref();

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
        let h = handle.0;
        if h == USB_VENDOR_HANDLE.0 {
            Some(VENDOR_ID)
        } else if h == USB_PRODUCT_HANDLE.0 {
            Some(self.product_desc_bytes)
        } else if h == USB_SERIAL_HANDLE.0 {
            Some(self.serial_desc_bytes)
        } else if h == USB_CDC_COMM_HANDLE.0 {
            Some(USB_COMM)
        } else if h == USB_CDC_DATA_HANDLE.0 {
            Some(USB_DATA)
        } else if h == DFU_FIRMWARE_HANDLE.0 {
            Some(DFU_FIRMWARE)
        } else if h == DFU_UDS_CERT_HANDLE.0 {
            Some(DFU_UDS_CERT)
        } else if h == DFU_CDI0_CERT_HANDLE.0 {
            Some(DFU_CDI0_CERT)
        } else if h == DFU_CDI1_CERT_HANDLE.0 {
            Some(DFU_CDI1_CERT)
        } else if h == DFU_SPI_EEPROM_HANDLE.0 {
            Some(DFU_SPI_EEPROM)
        } else {
            None
        }
    }
}

pub struct EepromDfuHandler {
    spi_flash: FlashIpcClient,
}

impl EepromDfuHandler {
    pub fn new(spi_flash: FlashIpcClient) -> Self {
        Self { spi_flash }
    }
}

impl DfuHandler for EepromDfuHandler {
    fn dnload(&mut self, alt_setting: u8, block_num: u16, data: &[u8]) -> Result<(), DfuStatus> {
        if data.is_empty() {
            return Ok(());
        }
        if alt_setting != DFU_ALT_SPI_EEPROM0 {
            return Err(DfuStatus::ErrTarget);
        }
        let address = (block_num as u32) * 2048;
        let (total_size, page_size, _) = self
            .spi_flash
            .geometry()
            .map_err(|_| DfuStatus::ErrUnknown)?;
        if address >= total_size.get() as u32 {
            return Err(DfuStatus::ErrAddress);
        }
        if (address as usize) % page_size.get() == 0 {
            self.spi_flash
                .erase(FlashAddress::new(address), page_size)
                .map_err(|_| DfuStatus::ErrErase)?;
        }
        self.spi_flash
            .program(FlashAddress::new(address), data)
            .map_err(|_| DfuStatus::ErrProg)
    }

    fn upload(
        &mut self,
        alt_setting: u8,
        block_num: u16,
        data: &mut [u8],
    ) -> Result<usize, DfuStatus> {
        if alt_setting != DFU_ALT_SPI_EEPROM0 {
            return Err(DfuStatus::ErrTarget);
        }
        let address = (block_num as u32) * 2048;
        let (total_size, _, _) = self
            .spi_flash
            .geometry()
            .map_err(|_| DfuStatus::ErrUnknown)?;
        if address >= total_size.get() as u32 {
            return Ok(0);
        }
        let len = data.len().min((total_size.get() as u32 - address) as usize);
        if len == 0 {
            return Ok(0);
        }
        self.spi_flash
            .read(FlashAddress::new(address), &mut data[..len])
            .map_err(|_| DfuStatus::ErrUnknown)?;
        Ok(len)
    }

    fn manifest(&mut self) -> Result<(), DfuStatus> {
        Ok(())
    }

    fn abort(&mut self) {}
}

fn handle_usb() -> Result<(), ErrorCode> {
    let lc_ctrl = unsafe { LcCtrl::new() };
    let device_id: [u32; 8] = lc_ctrl.regs().device_id().read().into();
    let mut serial_num_buffer = Aligned::<A4, _>([0_u8; 130]);
    let descriptors = MyDescriptors {
        serial_desc_bytes: hal_usb::hex_utf16_descriptor_aligned(
            &mut serial_num_buffer,
            device_id.as_bytes(),
        )
        .unwrap_or(PRODUCT_ID_DEFAULT),
        product_desc_bytes: PRODUCT_ID_DEFAULT,
    };

    const USB_CONFIG: UsbConfig = UsbConfig::new(&CDC_BUILDER.eps().0, &CDC_BUILDER.eps().1);

    let spi_flash = FlashIpcClient::new(IpcHandle::new(handle::SPI_FLASH_USB))?;

    let dfu_handler = EepromDfuHandler::new(spi_flash);
    let mut dfu = DfuClass::<_, 2048>::new(DFU_BUILDER, dfu_handler);

    let mut usb = usb_driver::Usb::new(unsafe { Usbdev::new() }, USB_CONFIG);
    let mut ep0 = usb_stack::SimpleEp0::new();
    let mut cdc_acm = CdcAcm::<256, 256>::new(CDC_BUILDER);

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
                | signals::USBDEV_AV_SETUP_EMPTY,
            Instant::MAX,
        )
        .map_err(ErrorCode::kernel_error)?;

        while let Some(event) = usb.poll() {
            let mut action = match cdc_acm.handle_event(event) {
                Ok(a) => a,
                Err(event) => match dfu.handle_event(event) {
                    Ok(a) => a,
                    Err(e) => ep0.handle_event(e, &descriptors).unwrap_or(UsbAction::None),
                },
            };
            action.run(&mut usb);
        }
        let _ = syscall::interrupt_ack(handle::USBDEV_INTERRUPTS, wait_return.pending_signals);

        while let Some(byte) = cdc_acm.rx_queue.pop() {
            let _ = cdc_acm.tx_queue.push(byte);
        }

        cdc_acm.poll_transmit(&mut usb);
        dfu.poll(&mut usb);
    }
}

fn usb_setup_pinmux() {
    use top_earlgrey::{PinmuxInsel, PinmuxPeripheralIn};
    let mut pinmux = unsafe { PinmuxAon::new() };

    pinmux
        .regs_mut()
        .mio_periph_insel()
        .at(PinmuxPeripheralIn::UsbdevSense as usize)
        .modify(|_| (PinmuxInsel::ConstantOne as u32).into());
}

fn usbmgr_server() -> Result<(), ErrorCode> {
    usb_setup_pinmux();
    handle_usb()
}

#[process_entry("usbmgr")]
fn entry() -> Result<(), Error> {
    let _ = usbmgr_server();
    loop {}
}
