// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP control protocol responder (message type `0x00`).
//!
//! This crate holds no transport. The caller registers the listener, receives
//! each request, calls [`ControlResponder::handle`], and sends the bytes
//! `handle` wrote when it returns `Ok`.
//!
//! Clause, table, and figure numbers in this crate refer to DSP0236 1.3.1.
//!
//! # Examples
//!
//! ```
//! use mctp_lib::config;
//! use openprot_mctp_api::{MctpClient, MctpError, Stack};
//! use openprot_mctp_control::{
//!     ControlConfig, ControlResponder, MAX_RESPONSE_LEN,
//!     MCTP_MSG_TYPE_CONTROL,
//! };
//!
//! fn serve<C: MctpClient>(stack: &Stack<C>) -> Result<(), MctpError> {
//!     use openprot_mctp_api::{MctpListener, MctpRespChannel};
//!
//!     let responder = ControlResponder::try_new(ControlConfig::simple_endpoint(&[]))
//!         .expect("an empty type list and the 1.3.1 tables fit");
//!     let mut listener = stack.listener(MCTP_MSG_TYPE_CONTROL, 0)?;
//!     let mut request_buf = [0u8; config::MAX_PAYLOAD];
//!     let mut response_buf = [0u8; MAX_RESPONSE_LEN];
//!
//!     loop {
//!         let (_meta, request, mut reply) = listener.recv(&mut request_buf)?;
//!         let handled = responder.handle(stack, request, &mut response_buf);
//!         // `Ok(None)` and an error each leave no reply to send.
//!         if let Ok(Some(len)) = handled {
//!             reply.send(&response_buf[..len])?;
//!         }
//!     }
//! }
//! ```

#![no_std]
#![deny(missing_docs)]

use mctp::Eid;
use mctp_lib::mctp_control::{
    CommandCode, CompletionCode, EidAllocationStatus, EidAssignmentStatus, EidType, EndpointType,
    GetEndpointIDResponse, GetMCTPVersionSupportResponse, GetMctpMessageTypeSupportResponse,
    MctpCodec, MctpControlHeader, MctpMessageType, SetEndpointIdResponse,
};
use openprot_mctp_api::{MctpClient, Stack};
use pw_status::Error;
use util_error::{ErrorCode, ErrorModule};

/// The MCTP message type that carries the control protocol (DSP0236, Table 3).
pub const MCTP_MSG_TYPE_CONTROL: u8 = 0x00;

/// The MCTP control responder error module.
pub const MCTP_CONTROL: ErrorModule = ErrorModule::new(0x4d43); // ascii `MC`
/// The request is too short to carry a control header.
pub const MCTP_CONTROL_MALFORMED: ErrorCode = MCTP_CONTROL.from_pw(1, Error::InvalidArgument);
/// The response buffer cannot hold the reply; see [`MAX_RESPONSE_LEN`].
pub const MCTP_CONTROL_RESPONSE_TOO_SHORT: ErrorCode =
    MCTP_CONTROL.from_pw(2, Error::ResourceExhausted);
/// A version table is not whole 4-byte codes or holds more than
/// [`MAX_VERSION_CODES`].
pub const MCTP_CONTROL_BAD_VERSION_TABLE: ErrorCode =
    MCTP_CONTROL.from_pw(3, Error::InvalidArgument);
/// The message type list holds more than [`MAX_MSG_TYPES`] entries.
pub const MCTP_CONTROL_TOO_MANY_MSG_TYPES: ErrorCode =
    MCTP_CONTROL.from_pw(4, Error::InvalidArgument);
/// The static EID is one no endpoint may hold (DSP0236, Table 14).
pub const MCTP_CONTROL_ILLEGAL_EID: ErrorCode = MCTP_CONTROL.from_pw(5, Error::InvalidArgument);
/// The stack refused to take the static EID.
pub const MCTP_CONTROL_SET_EID_FAILED: ErrorCode = MCTP_CONTROL.from_pw(6, Error::Unavailable);
/// A message type entry is control itself, past 7 bits, or listed twice.
pub const MCTP_CONTROL_BAD_MSG_TYPE: ErrorCode = MCTP_CONTROL.from_pw(7, Error::InvalidArgument);

/// Byte length of one 32-bit version code (DSP0236, clause 12.6.1).
pub const VERSION_CODE_LEN: usize = 4;

/// Response buffer size that fits every reply a valid [`ControlConfig`] draws.
///
/// The longest reply is Get Message Type Support for [`MAX_MSG_TYPES`] types.
pub const MAX_RESPONSE_LEN: usize = 2 + 2 + MAX_MSG_TYPES;

/// Largest message type count Get Message Type Support can report.
///
/// Clause 11.4 limits a control message to one packet whose payload fits the
/// 64-byte baseline transmission unit. The message type byte, the two-byte
/// control header, the completion code, and the count leave 59 bytes.
pub const MAX_MSG_TYPES: usize = 64 - 5;

/// Largest version-code count a requester using `mctp-lib` can decode.
///
/// Both constructors refuse a longer table.
pub const MAX_VERSION_CODES: usize = 8;

/// Version codes for MCTP base specification 1.3.1 (DSP0236, clause 12.6.2).
pub const BASE_VERSIONS_1_3_1: [u8; 16] = [
    0xf1, 0xf0, 0xff, 0x00, // 1.0, any update, no alpha
    0xf1, 0xf1, 0xff, 0x00, // 1.1, any update, no alpha
    0xf1, 0xf2, 0xff, 0x00, // 1.2, any update, no alpha
    0xf1, 0xf3, 0xf1, 0x00, // 1.3.1, no alpha
];

/// Version codes for the MCTP control protocol (DSP0236, clause 12.6.3).
pub const CONTROL_VERSIONS_1_3_1: [u8; 16] = BASE_VERSIONS_1_3_1;

/// One message type this endpoint answers, with its binding versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MsgTypeVersions<'a> {
    /// The message type number (DSP0236, Table 3).
    pub msg_type: u8,
    /// Version codes Get MCTP Version Support reports for this type, 4 bytes
    /// per entry, lowest first.
    ///
    /// Table 18 reports the binding specification's version where the type
    /// has one, as DSP0241 for PLDM, rather than the protocol's own.
    pub versions: &'a [u8],
}

/// What one endpoint reports about itself.
///
/// The binary's `main` fills this in, since only it knows the image's
/// message types and endpoint role.
#[derive(Debug, Clone, Copy)]
pub struct ControlConfig<'a> {
    /// Message types this endpoint answers besides control.
    ///
    /// Get Message Type Support lists each `msg_type` (DSP0236, Table 19), and
    /// Get MCTP Version Support answers each with its `versions`, so listing a
    /// type commits the image to answering both for it.
    pub msg_types: &'a [MsgTypeVersions<'a>],
    /// Version codes for the MCTP base specification, 4 bytes per entry.
    pub base_versions: &'a [u8],
    /// Version codes for the control protocol, 4 bytes per entry.
    pub control_versions: &'a [u8],
    /// Endpoint role Get Endpoint ID reports (DSP0236, Table 15).
    pub endpoint_type: EndpointType,
    /// Medium-specific byte Get Endpoint ID returns (DSP0236, Table 15).
    ///
    /// Table 15 requires `0x00` unless the transport binding defines a value.
    /// On SMBus/I2C, DSP0237 1.2.0 Table 4 defines bit 0 as fairness
    /// arbitration support, and [`Self::simple_endpoint`] sets `0` because
    /// this stack does not implement fairness arbitration.
    pub transport_specific: u8,
}

impl<'a> ControlConfig<'a> {
    /// Build a configuration for a simple endpoint reporting MCTP 1.3.1.
    ///
    /// The [`ControlResponder`] constructor decides whether the EID is static
    /// or dynamic.
    pub const fn simple_endpoint(msg_types: &'a [MsgTypeVersions<'a>]) -> Self {
        Self {
            msg_types,
            base_versions: &BASE_VERSIONS_1_3_1,
            control_versions: &CONTROL_VERSIONS_1_3_1,
            endpoint_type: EndpointType::SimpleEndpoint,
            transport_specific: 0,
        }
    }
}

/// The Set Endpoint ID operation a request selected (DSP0236, Table 14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SetEidOperation {
    /// Assign the given EID (operation `00b`).
    SetEid(u8),
    /// Assign the given EID regardless of the tracked bus (operation `01b`).
    ForceEid(u8),
    /// Restore the statically configured EID (operation `10b`).
    ResetEid,
    /// Set the discovered flag and leave the EID alone (operation `11b`).
    SetDiscoveredFlag,
}

/// Answers MCTP control requests for one endpoint.
///
/// The endpoint's current EID lives in the MCTP stack rather than here, so the
/// two cannot drift.
#[derive(Debug, Clone, Copy)]
pub struct ControlResponder<'a> {
    config: ControlConfig<'a>,
    static_eid: Option<u8>,
}

impl<'a> ControlResponder<'a> {
    /// Create a responder for a dynamic endpoint, holding no configured EID.
    ///
    /// # Errors
    ///
    /// Returns [`MCTP_CONTROL_BAD_VERSION_TABLE`],
    /// [`MCTP_CONTROL_TOO_MANY_MSG_TYPES`], or [`MCTP_CONTROL_BAD_MSG_TYPE`]
    /// for a `config` whose replies `mctp-lib` cannot encode or decode, or
    /// that no packet can carry.
    pub const fn try_new(config: ControlConfig<'a>) -> Result<Self, ErrorCode> {
        if let Err(e) = check_config(&config) {
            return Err(e);
        }
        Ok(Self {
            config,
            static_eid: None,
        })
    }

    /// Create a responder for an endpoint configured with a static EID.
    ///
    /// The constructor writes that EID to the stack. DSP0236 clause 12.3 makes
    /// the configured value the endpoint's initial default, so the stack must
    /// hold it before the first request arrives. Reset EID restores it.
    ///
    /// # Errors
    ///
    /// Returns the error [`Self::try_new`] returns for `config`,
    /// [`MCTP_CONTROL_ILLEGAL_EID`] for an EID no endpoint may hold, or
    /// [`MCTP_CONTROL_SET_EID_FAILED`] where the stack refuses the write.
    pub fn try_new_with_static_eid<C: MctpClient>(
        stack: &Stack<C>,
        config: ControlConfig<'a>,
        static_eid: u8,
    ) -> Result<Self, ErrorCode> {
        check_config(&config)?;
        if !is_legal_eid(static_eid) {
            return Err(MCTP_CONTROL_ILLEGAL_EID);
        }
        stack
            .set_eid(static_eid)
            .map_err(|_| MCTP_CONTROL_SET_EID_FAILED)?;
        Ok(Self {
            config,
            static_eid: Some(static_eid),
        })
    }

    /// Answer one control request.
    ///
    /// `request` is the payload a receive call produced and `response` takes
    /// the reply, both including the control header. Returns the reply length,
    /// or `None` for a response or a datagram, which draws no reply (DSP0236,
    /// clause 11.5).
    ///
    /// # Errors
    ///
    /// Returns [`MCTP_CONTROL_MALFORMED`] or
    /// [`MCTP_CONTROL_RESPONSE_TOO_SHORT`]. Either means `response` holds no
    /// reply to send, so the caller sends nothing. An unsupported command is
    /// not an error: it answers `ERROR_UNSUPPORTED_CMD`.
    pub fn handle<C: MctpClient>(
        &self,
        stack: &Stack<C>,
        request: &[u8],
        response: &mut [u8],
    ) -> Result<Option<usize>, ErrorCode> {
        let header = MctpControlHeader::decode(request).map_err(|_| MCTP_CONTROL_MALFORMED)?;
        let body = request
            .get(MctpControlHeader::MCTP_CODEC_MIN_SIZE..)
            .ok_or(MCTP_CONTROL_MALFORMED)?;

        // No requester waits for a reply here.
        if !header.request || header.datagram {
            return Ok(None);
        }

        let reply_header = MctpControlHeader::new(
            /* request= */ false,
            /* datagram= */ false,
            header.instance_id,
            header.command_code,
        );
        let header_len = reply_header
            .encode(response)
            .map_err(|_| MCTP_CONTROL_RESPONSE_TOO_SHORT)?;
        let body_buf = response
            .get_mut(header_len..)
            .ok_or(MCTP_CONTROL_RESPONSE_TOO_SHORT)?;

        let body_len = self.encode_body(stack, header.command_code, body, body_buf)?;

        Ok(Some(header_len + body_len))
    }

    fn encode_body<C: MctpClient>(
        &self,
        stack: &Stack<C>,
        command: CommandCode,
        body: &[u8],
        out: &mut [u8],
    ) -> Result<usize, ErrorCode> {
        match command {
            CommandCode::SetEndpointID => self.set_endpoint_id(stack, body, out),
            CommandCode::GetEndpointID => self.get_endpoint_id(stack, body, out),
            CommandCode::GetMCTPVersionSupport => self.version_support(body, out),
            CommandCode::GetMessageTypeSupport => self.message_type_support(body, out),
            _ => encode_error(CompletionCode::ErrorUnsupportedCmd, out),
        }
    }

    fn set_endpoint_id<C: MctpClient>(
        &self,
        stack: &Stack<C>,
        body: &[u8],
        out: &mut [u8],
    ) -> Result<usize, ErrorCode> {
        let Some(operation) = decode_set_eid(body) else {
            return encode_error(CompletionCode::ErrorInvalidLength, out);
        };

        let eid = match operation {
            // DSP0236 Table 14 makes Force EID equivalent to Set EID on an
            // endpoint reached through one bus.
            SetEidOperation::SetEid(eid) | SetEidOperation::ForceEid(eid) if is_legal_eid(eid) => {
                eid
            }
            SetEidOperation::SetEid(_) | SetEidOperation::ForceEid(_) => {
                return encode_error(CompletionCode::ErrorInvalidData, out);
            }
            SetEidOperation::ResetEid => match self.static_eid {
                Some(configured) => configured,
                None => return encode_error(CompletionCode::ErrorInvalidData, out),
            },
            // The discovered flag serves some transport bindings only, and
            // this crate keeps no such flag.
            SetEidOperation::SetDiscoveredFlag => {
                return encode_error(CompletionCode::ErrorInvalidData, out);
            }
        };

        // The reply is encoded first, so a buffer too short for it fails
        // before the stack moves. The write then lands before `handle`
        // returns, so the reply leaves from the address just assigned.
        let len = SetEndpointIdResponse::new(
            CompletionCode::Success,
            EidAssignmentStatus::Accepted,
            EidAllocationStatus::NoEidPoolUsed,
            Eid(eid),
            0,
        )
        .encode(out)
        .map_err(|_| MCTP_CONTROL_RESPONSE_TOO_SHORT)?;

        // The EID passed `is_legal_eid`, so a refusal is this endpoint's own
        // failure, which DSP0236 Table 13 answers with the generic ERROR.
        if stack.set_eid(eid).is_err() {
            return encode_error(CompletionCode::Error, out);
        }
        Ok(len)
    }

    fn get_endpoint_id<C: MctpClient>(
        &self,
        stack: &Stack<C>,
        body: &[u8],
        out: &mut [u8],
    ) -> Result<usize, ErrorCode> {
        // DSP0236 Table 15's request carries no data.
        if !body.is_empty() {
            return encode_error(CompletionCode::ErrorInvalidLength, out);
        }
        let current = stack.get_eid();
        let response = GetEndpointIDResponse::new(
            CompletionCode::Success,
            Eid(current),
            self.config.endpoint_type,
            self.eid_type(current),
            self.config.transport_specific,
        );
        response
            .encode(out)
            .map_err(|_| MCTP_CONTROL_RESPONSE_TOO_SHORT)
    }

    /// Which Endpoint ID Type value [`Self::get_endpoint_id`] reports.
    ///
    /// DSP0236 Table 15 recommends the matches-or-differs pair wherever an
    /// endpoint supports Reset EID, and a static endpoint here supports it.
    fn eid_type(&self, current: u8) -> EidType {
        match self.static_eid {
            None => EidType::DynamicEid,
            Some(configured) if configured == current => EidType::StaticEidConfigured,
            Some(_) => EidType::StaticEidAvailable,
        }
    }

    fn version_support(&self, body: &[u8], out: &mut [u8]) -> Result<usize, ErrorCode> {
        // DSP0236 Table 18's request carries exactly one byte.
        let &[raw] = body else {
            return encode_error(CompletionCode::ErrorInvalidLength, out);
        };

        let codes = match MctpMessageType::from(raw) {
            MctpMessageType::Mctp => self.config.base_versions,
            MctpMessageType::Control => self.config.control_versions,
            _ => match self.msg_type_versions(raw) {
                Some(versions) => versions,
                // 0x80 is Table 18's "message type number not supported".
                None => return encode_error(CompletionCode::CommandSpecific(0x80), out),
            },
        };

        let response = GetMCTPVersionSupportResponse::new(CompletionCode::Success, codes);
        response
            .encode(out)
            .map_err(|_| MCTP_CONTROL_RESPONSE_TOO_SHORT)
    }

    fn msg_type_versions(&self, msg_type: u8) -> Option<&'a [u8]> {
        let entries = self.config.msg_types;
        entries
            .iter()
            .find(|entry| entry.msg_type == msg_type)
            .map(|entry| entry.versions)
    }

    fn message_type_support(&self, body: &[u8], out: &mut [u8]) -> Result<usize, ErrorCode> {
        // DSP0236 Table 19's request carries no data.
        if !body.is_empty() {
            return encode_error(CompletionCode::ErrorInvalidLength, out);
        }
        // `mctp-lib` encodes the list from a byte slice. `check_config`
        // bounds the entry count by MAX_MSG_TYPES.
        let mut types = [0u8; MAX_MSG_TYPES];
        for (slot, entry) in types.iter_mut().zip(self.config.msg_types) {
            *slot = entry.msg_type;
        }
        let response = GetMctpMessageTypeSupportResponse::new(
            CompletionCode::Success,
            &types[..self.config.msg_types.len()],
        );
        response
            .encode(out)
            .map_err(|_| MCTP_CONTROL_RESPONSE_TOO_SHORT)
    }
}

/// Whether an EID may be assigned to an endpoint.
///
/// DSP0236 Table 14 marks the null EID `0x00` and the broadcast EID `0xff`
/// illegal for assignment, and clause 8.2 reserves 1 through 7.
const fn is_legal_eid(eid: u8) -> bool {
    eid > 7 && eid != 0xff
}

/// Refuse a [`ControlConfig`] whose replies `mctp-lib` cannot encode or
/// decode, or that no packet can carry.
///
/// A partial version code would panic in `mctp-lib`'s encoders, and a table
/// past [`MAX_VERSION_CODES`] would draw a reply its decoder refuses.
const fn check_config(config: &ControlConfig<'_>) -> Result<(), ErrorCode> {
    if !version_table_fits(config.base_versions) || !version_table_fits(config.control_versions) {
        return Err(MCTP_CONTROL_BAD_VERSION_TABLE);
    }
    let msg_types = config.msg_types;
    if msg_types.len() > MAX_MSG_TYPES {
        return Err(MCTP_CONTROL_TOO_MANY_MSG_TYPES);
    }
    // `while` rather than `for`, since a `const fn` cannot drive an iterator.
    // One bit per 7-bit type, so a type listed twice finds its bit set.
    let mut seen: u128 = 0;
    let mut i = 0;
    while i < msg_types.len() {
        let entry = &msg_types[i];
        // Table 3 numbers message types in 7 bits, and control answers from
        // `control_versions` alone. The 7-bit check also keeps the shift
        // below under 128.
        if entry.msg_type == MCTP_MSG_TYPE_CONTROL || entry.msg_type > 0x7f {
            return Err(MCTP_CONTROL_BAD_MSG_TYPE);
        }
        if entry.versions.is_empty() || !version_table_fits(entry.versions) {
            return Err(MCTP_CONTROL_BAD_VERSION_TABLE);
        }
        let bit = 1u128 << entry.msg_type;
        if seen & bit != 0 {
            return Err(MCTP_CONTROL_BAD_MSG_TYPE);
        }
        seen |= bit;
        i += 1;
    }
    Ok(())
}

const fn version_table_fits(codes: &[u8]) -> bool {
    codes.len() % VERSION_CODE_LEN == 0 && codes.len() <= MAX_VERSION_CODES * VERSION_CODE_LEN
}

/// Decode a Set Endpoint ID request body (DSP0236, Table 14).
///
/// Reads the operation from byte 1 before the EID in byte 2, which Reset EID
/// and Set Discovered Flag ignore. This crate decodes the body itself because
/// `mctp-lib`'s decoder validates byte 2 first and so rejects either request
/// when byte 2 is absent or not a valid EID.
///
/// Returns `None` for a body of the wrong length: an empty body, a body past
/// 2 bytes, or a Set EID or Force EID without byte 2.
fn decode_set_eid(body: &[u8]) -> Option<SetEidOperation> {
    if body.len() > 2 {
        return None;
    }
    let operation = body.first()? & 0b0000_0011;
    match operation {
        0x00 => Some(SetEidOperation::SetEid(*body.get(1)?)),
        0x01 => Some(SetEidOperation::ForceEid(*body.get(1)?)),
        0x02 => Some(SetEidOperation::ResetEid),
        0x03 => Some(SetEidOperation::SetDiscoveredFlag),
        _ => None,
    }
}

fn encode_error(code: CompletionCode, out: &mut [u8]) -> Result<usize, ErrorCode> {
    *out.first_mut().ok_or(MCTP_CONTROL_RESPONSE_TOO_SHORT)? = code.into();
    Ok(1)
}
