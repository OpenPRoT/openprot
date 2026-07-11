// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use aligned::{Aligned, A8};
use base64ct::{Base64, Encoding};
use bootinfo_codegen::handle;
use core::str;
use earlgrey_util::flash::EarlgreyFlashAddress;
use earlgrey_util::ret_ram::RetRam;
use earlgrey_util::tags::{BootSlot, HardenedBool, OwnershipState};
use earlgrey_util::{CheckDigest, PersoCertificate};
use hal_flash::{Flash, FlashAddress};
use pw_status::Error;
use services_flash_client::FlashIpcClient;
use sha2::{Digest, Sha256};
use userspace::{entry, syscall};
use util_error::ErrorCode;
use util_ipc::IpcHandle;
use util_misc::hexstr;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

const OWNERSHIP_KEY_ALG_ECDSA_P256: u32 = 0x36353250;
const OWNERSHIP_KEY_ALG_SPX_PURE: u32 = 0x75502b53;
const OWNERSHIP_KEY_ALG_SPX_PREHASH: u32 = 0x32532b53;
const OWNERSHIP_KEY_ALG_HYBRID_SPX_PURE: u32 = 0x75502b48;
const OWNERSHIP_KEY_ALG_HYBRID_SPX_PREHASH: u32 = 0x32532b48;

#[derive(Clone, Copy, FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct TlvHeader {
    pub tag: u32,
    pub length: u16,
    pub major: u8,
    pub minor: u8,
}

#[derive(Clone, Copy, FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct OwnerBlock {
    pub header: TlvHeader,
    pub config_version: u32,
    pub sram_exec_mode: u32,
    pub ownership_key_alg: u32,
    pub update_mode: u32,
    pub min_security_version_bl0: u32,
    pub lock_constraint: u32,
    pub device_id: [u32; 8],
    pub boot_svc_after_wakeup: u32,
    pub reserved: [u32; 15],
    pub owner_key: [u8; 96],
}

fn bootslot_str(s: BootSlot) -> &'static str {
    s.as_str()
}

fn ownership_state_str(s: OwnershipState) -> &'static str {
    match s {
        OwnershipState::Recovery => "Recovery",
        OwnershipState::LockedOwner => "LockedOwner",
        OwnershipState::UnlockedSelf => "UnlockedSelf",
        OwnershipState::UnlockedAny => "UnlockedAny",
        OwnershipState::UnlockedEndorsed => "UnlockedEndorsed",
        _ => "Invalid",
    }
}

fn hardened_bool_str(b: HardenedBool) -> &'static str {
    match b {
        HardenedBool::True => "True",
        HardenedBool::False => "False",
        _ => "Invalid",
    }
}

fn print_pem(name: &str, der: &[u8]) {
    // We print the certificate using raw `debug_log` calls so we don't have to
    // parse out the `pw_log` headers like `[DBG]` or `[INF]`.
    let _ = syscall::debug_log(b"-----BEGIN CERTIFICATE-----\r\n");
    let mut b64_buf = [0u8; 3072];
    match Base64::encode(der, &mut b64_buf) {
        Ok(b64_str) => {
            let _ = syscall::debug_log(b64_str.as_bytes());
            let _ = syscall::debug_log(b"\r\n");
        }
        Err(_) => {
            pw_log::error!("Failed to Base64 encode certificate '{}'", name);
        }
    }
    let _ = syscall::debug_log(b"-----END CERTIFICATE-----\r\n");
}

fn print_owner_info(owner: &OwnerBlock) {
    let mut alg_buf = [0u8; 4];
    alg_buf.copy_from_slice(&owner.ownership_key_alg.to_le_bytes());
    let alg_str = str::from_utf8(&alg_buf).unwrap_or("Invalid");

    let mut mode_buf = [0u8; 4];
    mode_buf.copy_from_slice(&owner.update_mode.to_le_bytes());
    let mode_str = str::from_utf8(&mode_buf).unwrap_or("Invalid");

    let mut tag_buf = [0u8; 4];
    tag_buf.copy_from_slice(&owner.header.tag.to_le_bytes());
    let tag_str = str::from_utf8(&tag_buf).unwrap_or("Invalid");

    pw_log::info!("Ownership Info from OWNER_PAGE_0:");
    pw_log::info!("  header.tag: {}", tag_str);
    pw_log::info!("  config_version: {}", owner.config_version);
    pw_log::info!(
        "  ownership_key_alg: {} (0x{:08x})",
        alg_str,
        owner.ownership_key_alg
    );
    pw_log::info!("  update_mode: {} (0x{:08x})", mode_str, owner.update_mode);
    pw_log::info!(
        "  min_security_version_bl0: 0x{:08x}",
        owner.min_security_version_bl0
    );

    let mut hex_buf = [0u8; 256];

    match owner.ownership_key_alg {
        OWNERSHIP_KEY_ALG_ECDSA_P256 => {
            let x = &owner.owner_key[0..32];
            let y = &owner.owner_key[32..64];
            pw_log::info!("  Ownership Key (ECDSA P256):");
            pw_log::info!(
                "    x: {}",
                hexstr(x, &mut hex_buf).unwrap_or("invalid buffer")
            );
            pw_log::info!(
                "    y: {}",
                hexstr(y, &mut hex_buf).unwrap_or("invalid buffer")
            );
        }
        OWNERSHIP_KEY_ALG_SPX_PURE | OWNERSHIP_KEY_ALG_SPX_PREHASH => {
            let data = &owner.owner_key[0..32];
            pw_log::info!("  Ownership Key (SPX+ / SLH-DSA):");
            pw_log::info!(
                "    data: {}",
                hexstr(data, &mut hex_buf).unwrap_or("invalid buffer")
            );
        }
        OWNERSHIP_KEY_ALG_HYBRID_SPX_PURE | OWNERSHIP_KEY_ALG_HYBRID_SPX_PREHASH => {
            let x = &owner.owner_key[0..32];
            let y = &owner.owner_key[32..64];
            let spx_data = &owner.owner_key[64..96];
            pw_log::info!("  Ownership Key (Hybrid P256 + SPX+):");
            pw_log::info!(
                "    x: {}",
                hexstr(x, &mut hex_buf).unwrap_or("invalid buffer")
            );
            pw_log::info!(
                "    y: {}",
                hexstr(y, &mut hex_buf).unwrap_or("invalid buffer")
            );
            pw_log::info!(
                "    spx_data: {}",
                hexstr(spx_data, &mut hex_buf).unwrap_or("invalid buffer")
            );
        }
        _ => {
            let raw_hex = hexstr(&owner.owner_key, &mut hex_buf).unwrap_or("invalid buffer");
            pw_log::info!("  Ownership Key (Raw / Unknown Alg):");
            pw_log::info!("    owner_key: {}", raw_hex);
        }
    }
}

fn read_dice_certificates(flash: &mut FlashIpcClient) {
    pw_log::info!("Reading DICE certificates from INFO partitions...");
    let mut buf = Aligned::<A8, [u8; 2048]>([0u8; 2048]);

    // Read Bank 0, Page 9 (UDS Cert / Factory Certs)
    match flash.read(FlashAddress::info(0, 9, 0), &mut *buf) {
        Ok(_) => {
            let mut data = &buf[..];
            while !data.is_empty() {
                match PersoCertificate::from_bytes(data) {
                    Ok((cert, rest)) => {
                        pw_log::info!(
                            "Found Certificate '{}' on Bank 0, Page 9 (obj_type: {}, obj_size: {})",
                            cert.name,
                            cert.obj_type.0,
                            cert.obj_size
                        );
                        print_pem(cert.name, cert.certificate);
                        data = rest;
                    }
                    Err(_) => break,
                }
            }
        }
        Err(e) => {
            pw_log::warn!(
                "Failed to read Bank 0, Page 9 (UDS): 0x{:08x}",
                u32::from(e) as u32
            );
        }
    }

    // Read Bank 1, Page 9 (CDI0, CDI1 Certs)
    match flash.read(FlashAddress::info(1, 9, 0), &mut *buf) {
        Ok(_) => {
            let mut data = &buf[..];
            while !data.is_empty() {
                match PersoCertificate::from_bytes(data) {
                    Ok((cert, rest)) => {
                        pw_log::info!(
                            "Found Certificate '{}' on Bank 1, Page 9 (obj_type: {}, obj_size: {})",
                            cert.name,
                            cert.obj_type.0,
                            cert.obj_size
                        );
                        print_pem(cert.name, cert.certificate);
                        data = rest;
                    }
                    Err(_) => break,
                }
            }
        }
        Err(e) => {
            pw_log::warn!(
                "Failed to read Bank 1, Page 9 (CDI): 0x{:08x}",
                u32::from(e) as u32
            );
        }
    }
}

fn read_ownership_page(flash: &mut FlashIpcClient) {
    pw_log::info!("Reading OWNER_PAGE_0 from Flash INFO (Bank 1, Page 2)...");
    let mut buf = Aligned::<A8, [u8; 2048]>([0u8; 2048]);
    match flash.read(FlashAddress::info(1, 2, 0), &mut *buf) {
        Ok(_) => match OwnerBlock::read_from_prefix(&*buf) {
            Ok((owner_block, _rest)) => {
                print_owner_info(&owner_block);
            }
            Err(_) => {
                pw_log::warn!("Failed to parse OwnerBlock from OWNER_PAGE_0");
            }
        },
        Err(e) => {
            pw_log::warn!(
                "Failed to read Bank 1, Page 2 (OWNER_PAGE_0): 0x{:08x}",
                u32::from(e) as u32
            );
        }
    }
}

fn read_retention_ram() {
    pw_log::info!("Reading Retention RAM structures...");
    let retram = unsafe { RetRam::mut_ref() };

    let is_valid = retram
        .boot_log
        .check_digest(|data| Sha256::digest(data).into());
    pw_log::info!(
        "check_digest: {}",
        if is_valid { "VALID" } else { "INVALID" }
    );

    let bl = &retram.boot_log;
    let mut tag_buf = [0u8; 4];
    tag_buf.copy_from_slice(&bl.identifier.to_le_bytes());
    let tag_str = str::from_utf8(&tag_buf).unwrap_or("Invalid");

    pw_log::info!("Boot Log Information:");
    pw_log::info!("  identifier: {}", tag_str);
    pw_log::info!("  chip_version: 0x{:016x}", bl.chip_version.get());
    pw_log::info!("  rom_ext_slot: {}", bootslot_str(bl.rom_ext_slot));
    pw_log::info!(
        "  rom_ext_major: {}, rom_ext_minor: {}",
        bl.rom_ext_major,
        bl.rom_ext_minor
    );
    pw_log::info!("  rom_ext_size: {} bytes", bl.rom_ext_size);
    pw_log::info!("  rom_ext_nonce: 0x{:016x}", bl.rom_ext_nonce.get());
    pw_log::info!("  bl0_slot: {}", bootslot_str(bl.bl0_slot));
    pw_log::info!(
        "  ownership_state: {}",
        ownership_state_str(bl.ownership_state)
    );
    pw_log::info!("  ownership_transfers: {}", bl.ownership_transfers);
    pw_log::info!("  rom_ext_min_sec_ver: {}", bl.rom_ext_min_sec_ver);
    pw_log::info!("  bl0_min_sec_ver: {}", bl.bl0_min_sec_ver);
    pw_log::info!("  primary_bl0_slot: {}", bootslot_str(bl.primary_bl0_slot));
    pw_log::info!(
        "  retention_ram_initialized: {}",
        hardened_bool_str(bl.retention_ram_initialized)
    );

    pw_log::info!("reset_reasons: 0x{:08x}", retram.reset_reasons);
    pw_log::info!(
        "last_shutdown_reason: 0x{:08x}",
        retram.last_shutdown_reason.0
    );
}

fn handle_bootinfo() -> Result<(), ErrorCode> {
    let flash_channel = IpcHandle::new(handle::FLASH_SERVICE);
    let mut flash = FlashIpcClient::new(flash_channel)?;

    read_retention_ram();
    read_dice_certificates(&mut flash);
    read_ownership_page(&mut flash);

    Ok(())
}

#[entry]
fn entry() -> Result<(), Error> {
    pw_log::info!("🔄 RUNNING bootinfo test");
    let ret = handle_bootinfo();

    let ret = match ret {
        Ok(()) => {
            pw_log::info!("✅ PASSED bootinfo test");
            Ok(())
        }
        Err(e) => {
            pw_log::error!("❌ FAILED bootinfo test: {:08x}", u32::from(e) as u32);
            Err(Error::Unknown)
        }
    };

    let _ = syscall::debug_shutdown(ret);
    loop {}
}

util_panic::make_panic_handler!();
