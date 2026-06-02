// Licensed under the Apache-2.0 license

//! I3C Common Command Codes (CCC)
//!
//! Functions and types for executing I3C CCCs.

use super::config::I3cConfig;
use super::constants::{
    I3C_CCC_GETBCR, I3C_CCC_GETPID, I3C_CCC_GETSTATUS, I3C_CCC_RSTDAA, I3C_CCC_SETNEWDA,
};
use super::error::{CccErrorKind, I3cError};
use super::hardware::HardwareInterface;

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
    .map_err(|_| I3cError::CccError(CccErrorKind::Invalid))
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

    hw.do_ccc(config, &mut payload)
        .map_err(|_| I3cError::CccError(CccErrorKind::Invalid))
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

    hw.do_ccc(config, &mut payload)
        .map_err(|_| I3cError::CccError(CccErrorKind::Invalid))
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

    hw.do_ccc(config, &mut payload)
        .map_err(|_| I3cError::CccError(CccErrorKind::Invalid))?;

    Ok(bcr_buf[0])
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
    if curr_da == 0 || new_da == 0 {
        return Err(I3cError::CccError(CccErrorKind::InvalidParam));
    }

    let pos = config.attached.pos_of_addr(curr_da);
    if pos.is_none() {
        return Err(I3cError::CccError(CccErrorKind::NotFound));
    }

    if !config.addrbook.is_free(new_da) {
        return Err(I3cError::CccError(CccErrorKind::NoFreeSlot));
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

    hw.do_ccc(config, &mut payload)
        .map_err(|_| I3cError::CccError(CccErrorKind::Invalid))
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

    hw.do_ccc(config, &mut payload)
        .map_err(|_| I3cError::CccError(CccErrorKind::Invalid))?;

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

    hw.do_ccc(config, &mut payload)
        .map_err(|_| I3cError::CccError(CccErrorKind::Invalid))?;

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
        _ => Err(I3cError::CccError(CccErrorKind::Invalid)),
    }
}

/// Reset dynamic address assignment for all devices (broadcast)
pub fn ccc_rstdaa_all<H>(hw: &mut H, config: &mut I3cConfig) -> Result<(), I3cError>
where
    H: HardwareInterface,
{
    hw.do_ccc(
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
    .map_err(|_| I3cError::CccError(CccErrorKind::Invalid))
}
