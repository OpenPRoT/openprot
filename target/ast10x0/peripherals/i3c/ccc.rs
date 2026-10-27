// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C Common Command Codes (CCC)
//!
//! Functions and types for executing I3C CCCs.

use super::config::{DaState, I3cConfig, IbiState};
use super::constants::{
    I3C_BCR_IBI_PAYLOAD_HAS_DATA_BYTE, I3C_CCC_GETBCR, I3C_CCC_GETDCR, I3C_CCC_GETMRL,
    I3C_CCC_GETMWL, I3C_CCC_GETMXDS, I3C_CCC_GETPID, I3C_CCC_GETSTATUS, I3C_CCC_RSTDAA,
    I3C_CCC_SETMRL, I3C_CCC_SETMRL_BC, I3C_CCC_SETMWL, I3C_CCC_SETMWL_BC, I3C_CCC_SETNEWDA,
};
use super::error::{CccErrorKind, I3cError};
use super::hardware::HardwareInterface;
use super::types::DevKind;

// =============================================================================
// CCC Types
// =============================================================================

/// CCC target payload for direct CCCs
#[derive(Debug)]
pub struct CccTargetPayload<'a> {
    /// Target 7-bit dynamic address
    pub addr: u8,
    /// `false` = write, `true` = read
    pub rnw: bool,
    /// Data buffer for write (source) or read (destination)
    pub data: Option<&'a mut [u8]>,
    /// Actual bytes transferred (driver fills on return)
    pub num_xfer: usize,
}

/// CCC descriptor
#[derive(Debug)]
pub struct Ccc<'a> {
    /// CCC ID (command code)
    pub id: u8,
    /// Optional CCC data immediately following the CCC byte
    pub data: Option<&'a mut [u8]>,
    /// Actual bytes transferred (driver fills on return)
    pub num_xfer: usize,
}

/// Complete CCC transaction description
#[derive(Debug)]
pub struct CccPayload<'a, 'b> {
    /// The CCC command
    pub ccc: Option<Ccc<'a>>,
    /// Optional list of direct-CCC target payloads
    pub targets: Option<&'b mut [CccTargetPayload<'a>]>,
}

// =============================================================================
// CCC Reset Action
// =============================================================================

/// RSTACT defining byte values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CccRstActDefByte {
    /// No reset
    NoReset = 0x0,
    /// Reset peripheral only
    PeriphralOnly = 0x1,
    /// Reset whole target
    ResetWholeTarget = 0x2,
    /// Debug network adapter
    DebugNetworkAdapter = 0x3,
    /// Virtual target detect
    VirtualTargetDetect = 0x4,
}

impl CccRstActDefByte {
    #[inline]
    fn as_byte(self) -> u8 {
        self as u8
    }
}

// =============================================================================
// GETSTATUS Types
// =============================================================================

/// GETSTATUS format selection
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GetStatusFormat {
    /// Format 1 (no defining byte)
    Fmt1,
    /// Format 2 (with defining byte)
    Fmt2(GetStatusDefByte),
}

/// GETSTATUS defining byte values
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GetStatusDefByte {
    /// 0x00 - TGTSTAT
    TgtStat,
    /// 0x91 - PRECR
    Precr,
}

impl GetStatusDefByte {
    #[inline]
    fn as_byte(self) -> u8 {
        match self {
            Self::TgtStat => 0x00,
            Self::Precr => 0x91,
        }
    }
}

/// GETSTATUS response
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GetStatusResp {
    /// Format 1 response
    Fmt1 { status: u16 },
    /// Format 2 response
    Fmt2 {
        kind: GetStatusDefByte,
        raw_u16: u16,
    },
}

// =============================================================================
// CCC Helper Functions
// =============================================================================

const fn ccc_enec(broadcast: bool) -> u8 {
    if broadcast {
        0x00
    } else {
        0x80
    }
}

const fn ccc_disec(broadcast: bool) -> u8 {
    if broadcast {
        0x01
    } else {
        0x81
    }
}

const fn ccc_rstact(broadcast: bool) -> u8 {
    if broadcast {
        0x2a
    } else {
        0x9a
    }
}

// =============================================================================
// CCC Operations
// =============================================================================

/// Preserve errors with ambiguous bus-side effects.
fn map_ccc_err(e: I3cError) -> I3cError {
    match e {
        I3cError::Timeout | I3cError::RespError | I3cError::IoError | I3cError::AddressNack => e,
        _ => I3cError::CccError(CccErrorKind::Invalid),
    }
}

fn mark_setnewda_unknown(config: &mut I3cConfig, dev_idx: usize, new_da: u8) {
    config.mark_da_unknown(dev_idx, Some(new_da));
    if let Some(dev) = config.attached.devices.get_mut(dev_idx) {
        dev.ibi_state = IbiState::Unknown;
    }
}

/// Enable/disable events for all devices (broadcast)
pub fn ccc_events_all_set<H>(
    hw: &mut H,
    config: &mut I3cConfig,
    enable: bool,
    events: u8,
) -> Result<(), I3cError>
where
    H: HardwareInterface,
{
    // SIR changes require per-device DAT updates.
    if events & super::constants::I3C_CCC_EVT_INTR != 0 {
        return Err(I3cError::Access);
    }

    let id = if enable {
        ccc_enec(true)
    } else {
        ccc_disec(true)
    };

    hw.do_ccc(
        config,
        &mut CccPayload {
            ccc: Some(Ccc {
                id,
                data: Some(&mut [events]),
                num_xfer: 0,
            }),
            targets: None,
        },
    )
    .map_err(map_ccc_err)
}

/// Enable/disable events for a specific device (direct)
pub fn ccc_events_set<H>(
    hw: &mut H,
    config: &mut I3cConfig,
    da: u8,
    enable: bool,
    events: u8,
) -> Result<(), I3cError>
where
    H: HardwareInterface,
{
    if da == 0 {
        return Err(I3cError::CccError(CccErrorKind::InvalidParam));
    }

    let mut ev_buf = [events];
    let tgt = CccTargetPayload {
        addr: da,
        rnw: false,
        data: Some(&mut ev_buf[..]),
        num_xfer: 0,
    };

    let mut tgts = [tgt];
    let ccc_id = if enable {
        ccc_enec(false)
    } else {
        ccc_disec(false)
    };
    let ccc = Ccc {
        id: ccc_id,
        data: None,
        num_xfer: 0,
    };

    let mut payload = CccPayload {
        ccc: Some(ccc),
        targets: Some(&mut tgts[..]),
    };

    hw.do_ccc(config, &mut payload).map_err(map_ccc_err)
}

/// Execute RSTACT (Reset Action) broadcast
pub fn ccc_rstact_all<H>(
    hw: &mut H,
    config: &mut I3cConfig,
    action: CccRstActDefByte,
) -> Result<(), I3cError>
where
    H: HardwareInterface,
{
    let mut db = [action.as_byte()];
    let ccc = Ccc {
        id: ccc_rstact(true),
        data: Some(&mut db[..]),
        num_xfer: 0,
    };
    let mut payload = CccPayload {
        ccc: Some(ccc),
        targets: None,
    };

    hw.do_ccc(config, &mut payload).map_err(map_ccc_err)
}

/// Get BCR (Bus Characteristics Register) from a device
pub fn ccc_getbcr<H>(hw: &mut H, config: &mut I3cConfig, dyn_addr: u8) -> Result<u8, I3cError>
where
    H: HardwareInterface,
{
    if dyn_addr == 0 {
        return Err(I3cError::CccError(CccErrorKind::InvalidParam));
    }

    let mut bcr_buf = [0u8; 1];

    let tgt = CccTargetPayload {
        addr: dyn_addr,
        rnw: true,
        data: Some(&mut bcr_buf[..]),
        num_xfer: 0,
    };
    let mut tgts = [tgt];

    let ccc = Ccc {
        id: I3C_CCC_GETBCR,
        data: None,
        num_xfer: 0,
    };
    let mut payload = CccPayload {
        ccc: Some(ccc),
        targets: Some(&mut tgts[..]),
    };

    hw.do_ccc(config, &mut payload).map_err(map_ccc_err)?;

    Ok(bcr_buf[0])
}

/// Get DCR (Device Characteristics Register) from a device
pub fn ccc_getdcr<H>(hw: &mut H, config: &mut I3cConfig, dyn_addr: u8) -> Result<u8, I3cError>
where
    H: HardwareInterface,
{
    if dyn_addr == 0 {
        return Err(I3cError::CccError(CccErrorKind::InvalidParam));
    }

    let mut dcr_buf = [0u8; 1];

    let tgt = CccTargetPayload {
        addr: dyn_addr,
        rnw: true,
        data: Some(&mut dcr_buf[..]),
        num_xfer: 0,
    };
    let mut tgts = [tgt];

    let ccc = Ccc {
        id: I3C_CCC_GETDCR,
        data: None,
        num_xfer: 0,
    };
    let mut payload = CccPayload {
        ccc: Some(ccc),
        targets: Some(&mut tgts[..]),
    };

    hw.do_ccc(config, &mut payload).map_err(map_ccc_err)?;

    Ok(dcr_buf[0])
}

/// Send SETNEWDA without changing bookkeeping or DAT state.
pub(crate) fn ccc_setnewda_bus_only<H>(
    hw: &mut H,
    config: &mut I3cConfig,
    curr_da: u8,
    new_da: u8,
) -> Result<(), I3cError>
where
    H: HardwareInterface,
{
    // Reject addresses that would be truncated on the bus.
    if curr_da == 0 || new_da == 0 || new_da >= super::constants::I3C_BROADCAST_ADDR {
        return Err(I3cError::CccError(CccErrorKind::InvalidParam));
    }

    let mut new_dyn_addr = (new_da & 0x7F) << 1;
    let tgt = CccTargetPayload {
        addr: curr_da,
        rnw: false,
        data: Some(core::slice::from_mut(&mut new_dyn_addr)),
        num_xfer: 0,
    };
    let mut tgts = [tgt];
    let ccc = Ccc {
        id: I3C_CCC_SETNEWDA,
        data: None,
        num_xfer: 0,
    };
    let mut payload = CccPayload {
        ccc: Some(ccc),
        targets: Some(&mut tgts[..]),
    };

    hw.do_ccc(config, &mut payload).map_err(map_ccc_err)
}

/// Set new dynamic address for a device
pub fn ccc_setnewda<H>(
    hw: &mut H,
    config: &mut I3cConfig,
    curr_da: u8,
    new_da: u8,
) -> Result<(), I3cError>
where
    H: HardwareInterface,
{
    let Some(dev_idx) = config.attached.find_dev_idx_by_addr(curr_da) else {
        return Err(I3cError::CccError(CccErrorKind::NotFound));
    };
    let Some(dev) = config.attached.devices.get(dev_idx) else {
        return Err(I3cError::CccError(CccErrorKind::NotFound));
    };
    if dev.kind != DevKind::I3c || dev.da_state != DaState::Verified {
        return Err(I3cError::Access);
    }
    let Some(pos) = config.attached.pos_of(dev_idx) else {
        return Err(I3cError::CccError(CccErrorKind::NotFound));
    };
    let ibi_was_enabled = config
        .attached
        .devices
        .iter()
        .find(|dev| dev.dyn_addr == curr_da)
        .is_some_and(|dev| dev.ibi_state == IbiState::Enabled);

    // Check reservations before sending SETNEWDA.
    if new_da != curr_da && !config.addrbook.is_free(new_da) {
        return Err(I3cError::CccError(CccErrorKind::NoFreeSlot));
    }

    if let Err(e) = ccc_setnewda_bus_only(hw, config, curr_da, new_da) {
        if matches!(e, I3cError::Timeout | I3cError::RespError) {
            // The target may have moved despite the error.
            mark_setnewda_unknown(config, dev_idx, new_da);
        }
        return Err(e);
    }

    // Reprogram DAT before committing bookkeeping.
    if hw.attach_i3c_dev(pos.into(), new_da).is_err() {
        // The target moved but DAT did not; block further transfers.
        mark_setnewda_unknown(config, dev_idx, new_da);
        hw.mark_xfer_faulted();
        return Err(I3cError::CccError(CccErrorKind::Invalid));
    }

    // Commit the move and restore IBI state if needed.
    if config.reassign_da(curr_da, new_da).is_err() {
        // Bus and DAT moved but bookkeeping did not.
        mark_setnewda_unknown(config, dev_idx, new_da);
        hw.mark_xfer_faulted();
        return Err(I3cError::CccError(CccErrorKind::Invalid));
    }

    if ibi_was_enabled {
        let result = hw.ibi_enable(config, new_da);
        if let Some(dev) = config
            .attached
            .devices
            .iter_mut()
            .find(|dev| dev.dyn_addr == new_da)
        {
            dev.ibi_state = match result {
                Ok(()) => IbiState::Enabled,
                Err(I3cError::Timeout | I3cError::RespError) => IbiState::Unknown,
                Err(_) => IbiState::Disabled,
            };
        }
        result.map_err(map_ccc_err)?;
    }
    Ok(())
}

/// Send a direct write CCC with a small fixed payload.
fn ccc_direct_write<H>(
    hw: &mut H,
    config: &mut I3cConfig,
    id: u8,
    da: u8,
    payload: &mut [u8],
) -> Result<(), I3cError>
where
    H: HardwareInterface,
{
    if da == 0 {
        return Err(I3cError::CccError(CccErrorKind::InvalidParam));
    }
    let tgt = CccTargetPayload {
        addr: da,
        rnw: false,
        data: Some(payload),
        num_xfer: 0,
    };
    let mut tgts = [tgt];
    let mut p = CccPayload {
        ccc: Some(Ccc {
            id,
            data: None,
            num_xfer: 0,
        }),
        targets: Some(&mut tgts[..]),
    };
    hw.do_ccc(config, &mut p).map_err(map_ccc_err)
}

/// Send a direct read CCC into a small fixed buffer.
fn ccc_direct_read<H>(
    hw: &mut H,
    config: &mut I3cConfig,
    id: u8,
    da: u8,
    out: &mut [u8],
) -> Result<(), I3cError>
where
    H: HardwareInterface,
{
    if da == 0 {
        return Err(I3cError::CccError(CccErrorKind::InvalidParam));
    }
    let tgt = CccTargetPayload {
        addr: da,
        rnw: true,
        data: Some(out),
        num_xfer: 0,
    };
    let mut tgts = [tgt];
    let mut p = CccPayload {
        ccc: Some(Ccc {
            id,
            data: None,
            num_xfer: 0,
        }),
        targets: Some(&mut tgts[..]),
    };
    hw.do_ccc(config, &mut p).map_err(map_ccc_err)
}

/// Set Maximum Write Length for a device (direct SETMWL); mirrors the value
/// into the attached-device entry on success.
pub fn ccc_setmwl<H>(hw: &mut H, config: &mut I3cConfig, da: u8, mwl: u16) -> Result<(), I3cError>
where
    H: HardwareInterface,
{
    let mut payload = mwl.to_be_bytes();
    ccc_direct_write(hw, config, I3C_CCC_SETMWL, da, &mut payload)?;
    if let Some(idx) = config.attached.find_dev_idx_by_addr(da)
        && let Some(dev) = config.attached.devices.get_mut(idx)
    {
        dev.mwl = mwl;
    }
    Ok(())
}

/// Set Maximum Read Length for a device (direct SETMRL). `ibi_len` adds the
/// optional third byte (max IBI payload size) for targets whose BCR
/// advertises an IBI payload. Mirrors the values into the attached-device
/// entry on success.
pub fn ccc_setmrl<H>(
    hw: &mut H,
    config: &mut I3cConfig,
    da: u8,
    mrl: u16,
    ibi_len: Option<u8>,
) -> Result<(), I3cError>
where
    H: HardwareInterface,
{
    let be = mrl.to_be_bytes();
    let mut buf3 = [be[0], be[1], 0];
    let payload: &mut [u8] = match ibi_len {
        Some(n) => {
            buf3[2] = n;
            &mut buf3[..3]
        }
        None => &mut buf3[..2],
    };
    ccc_direct_write(hw, config, I3C_CCC_SETMRL, da, payload)?;
    if let Some(idx) = config.attached.find_dev_idx_by_addr(da)
        && let Some(dev) = config.attached.devices.get_mut(idx)
    {
        dev.mrl = mrl;
        if let Some(n) = ibi_len {
            dev.max_ibi = n;
        }
    }
    Ok(())
}

/// Broadcast SETMWL to all devices.
pub fn ccc_setmwl_all<H>(hw: &mut H, config: &mut I3cConfig, mwl: u16) -> Result<(), I3cError>
where
    H: HardwareInterface,
{
    let mut payload = mwl.to_be_bytes();
    hw.do_ccc(
        config,
        &mut CccPayload {
            ccc: Some(Ccc {
                id: I3C_CCC_SETMWL_BC,
                data: Some(&mut payload[..]),
                num_xfer: 0,
            }),
            targets: None,
        },
    )
    .map_err(map_ccc_err)?;

    for dev in &mut config.attached.devices {
        if dev.kind == DevKind::I3c {
            dev.mwl = mwl;
        }
    }
    Ok(())
}

/// Broadcast SETMRL to all devices.
pub fn ccc_setmrl_all<H>(
    hw: &mut H,
    config: &mut I3cConfig,
    mrl: u16,
    ibi_len: Option<u8>,
) -> Result<(), I3cError>
where
    H: HardwareInterface,
{
    let be = mrl.to_be_bytes();
    let mut buf3 = [be[0], be[1], 0];
    let payload: &mut [u8] = match ibi_len {
        Some(n) => {
            buf3[2] = n;
            &mut buf3[..3]
        }
        None => &mut buf3[..2],
    };
    hw.do_ccc(
        config,
        &mut CccPayload {
            ccc: Some(Ccc {
                id: I3C_CCC_SETMRL_BC,
                data: Some(payload),
                num_xfer: 0,
            }),
            targets: None,
        },
    )
    .map_err(map_ccc_err)?;

    for dev in &mut config.attached.devices {
        if dev.kind == DevKind::I3c {
            dev.mrl = mrl;
            if let Some(n) = ibi_len {
                dev.max_ibi = n;
            }
        }
    }
    Ok(())
}

/// Get Maximum Write Length from a device (GETMWL); mirrors the value into
/// the attached-device entry.
pub fn ccc_getmwl<H>(hw: &mut H, config: &mut I3cConfig, da: u8) -> Result<u16, I3cError>
where
    H: HardwareInterface,
{
    let mut buf = [0u8; 2];
    ccc_direct_read(hw, config, I3C_CCC_GETMWL, da, &mut buf)?;
    let mwl = u16::from_be_bytes(buf);
    if let Some(idx) = config.attached.find_dev_idx_by_addr(da)
        && let Some(dev) = config.attached.devices.get_mut(idx)
    {
        dev.mwl = mwl;
    }
    Ok(mwl)
}

/// Get Maximum Read Length from a device (GETMRL).
///
/// Returns `(mrl, max_ibi_len)`; the third response byte is present only for
/// targets whose BCR advertises an IBI payload (the attached entry's BCR
/// decides how many bytes are requested). Mirrors the values into the
/// attached-device entry.
pub fn ccc_getmrl<H>(
    hw: &mut H,
    config: &mut I3cConfig,
    da: u8,
) -> Result<(u16, Option<u8>), I3cError>
where
    H: HardwareInterface,
{
    let has_ibi_byte = config
        .attached
        .find_dev_idx_by_addr(da)
        .and_then(|idx| config.attached.devices.get(idx))
        .map(|d| u32::from(d.bcr) & I3C_BCR_IBI_PAYLOAD_HAS_DATA_BYTE != 0)
        .unwrap_or(false);

    let mut buf = [0u8; 3];
    let want = if has_ibi_byte { 3 } else { 2 };
    // `want` is 2 or 3, always within the buffer.
    let out = buf.get_mut(..want).ok_or(I3cError::Invalid)?;
    ccc_direct_read(hw, config, I3C_CCC_GETMRL, da, out)?;

    let mrl = u16::from_be_bytes([buf[0], buf[1]]);
    let ibi_len = has_ibi_byte.then_some(buf[2]);
    if let Some(idx) = config.attached.find_dev_idx_by_addr(da)
        && let Some(dev) = config.attached.devices.get_mut(idx)
    {
        dev.mrl = mrl;
        if let Some(n) = ibi_len {
            dev.max_ibi = n;
        }
    }
    Ok((mrl, ibi_len))
}

/// Get Max Data Speed from a device (GETMXDS format 1).
///
/// Returns `(max_wr, max_rd)` raw speed bytes; mirrored into the
/// attached-device entry.
pub fn ccc_getmxds<H>(hw: &mut H, config: &mut I3cConfig, da: u8) -> Result<(u8, u8), I3cError>
where
    H: HardwareInterface,
{
    let mut buf = [0u8; 2];
    ccc_direct_read(hw, config, I3C_CCC_GETMXDS, da, &mut buf)?;
    let (max_wr, max_rd) = (buf[0], buf[1]);
    if let Some(idx) = config.attached.find_dev_idx_by_addr(da)
        && let Some(dev) = config.attached.devices.get_mut(idx)
    {
        dev.maxwr = max_wr;
        dev.maxrd = max_rd;
    }
    Ok((max_wr, max_rd))
}

fn bytes_to_pid(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .take(6)
        .fold(0u64, |acc, &b| (acc << 8) | u64::from(b))
}

/// Get PID (Provisional ID) from a device
pub fn ccc_getpid<H>(hw: &mut H, config: &mut I3cConfig, dyn_addr: u8) -> Result<u64, I3cError>
where
    H: HardwareInterface,
{
    let mut pid_buf = [0u8; 6];

    let tgt = CccTargetPayload {
        addr: dyn_addr,
        rnw: true,
        data: Some(&mut pid_buf[..]),
        num_xfer: 0,
    };
    let mut tgts = [tgt];

    let ccc = Ccc {
        id: I3C_CCC_GETPID,
        data: None,
        num_xfer: 0,
    };
    let mut payload = CccPayload {
        ccc: Some(ccc),
        targets: Some(&mut tgts[..]),
    };

    hw.do_ccc(config, &mut payload).map_err(map_ccc_err)?;

    Ok(bytes_to_pid(&pid_buf))
}

/// Get status from a device
pub fn ccc_getstatus<H>(
    hw: &mut H,
    config: &mut I3cConfig,
    da: u8,
    fmt: GetStatusFormat,
) -> Result<GetStatusResp, I3cError>
where
    H: HardwareInterface,
{
    let mut data_buf = [0u8; 2];
    let mut defbyte_buf = [0u8; 1];

    let tgt = CccTargetPayload {
        addr: da,
        rnw: true,
        data: Some(&mut data_buf[..]),
        num_xfer: 0,
    };

    let mut ccc = Ccc {
        id: I3C_CCC_GETSTATUS,
        data: None,
        num_xfer: 0,
    };

    let kind_opt = match fmt {
        GetStatusFormat::Fmt1 => None,
        GetStatusFormat::Fmt2(kind) => {
            defbyte_buf[0] = kind.as_byte();
            ccc.data = Some(&mut defbyte_buf[..]);
            Some(kind)
        }
    };

    let mut targets_arr = [tgt];
    let mut payload = CccPayload {
        ccc: Some(ccc),
        targets: Some(&mut targets_arr[..]),
    };

    hw.do_ccc(config, &mut payload).map_err(map_ccc_err)?;

    let val = u16::from_be_bytes(data_buf);

    let resp = match kind_opt {
        None => GetStatusResp::Fmt1 { status: val },
        Some(kind) => GetStatusResp::Fmt2 { kind, raw_u16: val },
    };
    Ok(resp)
}

/// Get status (Format 1) from a device
pub fn ccc_getstatus_fmt1<H>(hw: &mut H, config: &mut I3cConfig, da: u8) -> Result<u16, I3cError>
where
    H: HardwareInterface,
{
    match ccc_getstatus(hw, config, da, GetStatusFormat::Fmt1) {
        Ok(GetStatusResp::Fmt1 { status }) => Ok(status),
        // Fmt1 cannot produce Fmt2.
        Ok(GetStatusResp::Fmt2 { .. }) => Err(I3cError::CccError(CccErrorKind::Invalid)),
        Err(e) => Err(e),
    }
}

/// Reset dynamic address assignment for all devices (broadcast)
pub fn ccc_rstdaa_all<H>(hw: &mut H, config: &mut I3cConfig) -> Result<(), I3cError>
where
    H: HardwareInterface,
{
    let result = hw
        .do_ccc(
            config,
            &mut CccPayload {
                ccc: Some(Ccc {
                    id: I3C_CCC_RSTDAA,
                    data: None,
                    num_xfer: 0,
                }),
                targets: None,
            },
        )
        .map_err(map_ccc_err);

    match result {
        Ok(()) => {
            config.commit_rstdaa();
            Ok(())
        }
        Err(e @ (I3cError::Timeout | I3cError::RespError)) => {
            // The command may have reached none, some, or all targets. Keep
            // every possible owner reserved until a confirmed RSTDAA.
            config.mark_rstdaa_unknown();
            Err(e)
        }
        Err(e) => Err(e),
    }
}
