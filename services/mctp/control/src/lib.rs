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
//!     let responder =
//!         ControlResponder::try_new(ControlConfig::simple_endpoint(&[]))?;
//!     let mut listener = stack.listener(MCTP_MSG_TYPE_CONTROL, 0)?;
//!     let mut request_buf = [0u8; config::MAX_PAYLOAD];
//!     let mut response_buf = [0u8; MAX_RESPONSE_LEN];
//!
//!     loop {
//!         let (_meta, request, mut reply) = listener.recv(&mut request_buf)?;
//!         let handled = responder.handle(stack, request, &mut response_buf);
//!         // A `ControlError` means `response_buf` holds no reply to send.
//!         if let Ok(len) = handled {
//!             reply.send(&response_buf[..len])?;
//!         }
//!     }
//! }
//! ```

#![no_std]
#![deny(missing_docs)]

use core::fmt;

use mctp::Eid;
use mctp_lib::mctp_control::{
    CommandCode, CompletionCode, EidAllocationStatus, EidAssignmentStatus, EidType, EndpointType,
    GetEndpointIDResponse, GetMCTPVersionSupportResponse, GetMctpMessageTypeSupportResponse,
    MctpCodec, MctpControlHeader, MctpMessageType, SetEndpointIdResponse,
};
use openprot_mctp_api::{MctpClient, MctpError, ResponseCode, Stack};

/// The MCTP message type that carries the control protocol (DSP0236, Table 3).
pub const MCTP_MSG_TYPE_CONTROL: u8 = 0x00;

/// Byte length of one 32-bit version code (DSP0236, clause 12.6.1).
pub const VERSION_CODE_LEN: usize = 4;

/// Response buffer size that fits every reply for up to 32 message types.
///
/// Get Message Type Support grows with
/// [`ControlConfig::supported_msg_types`], so a list past 32 entries needs a
/// larger buffer, and `handle` returns [`ControlError::ResponseTooShort`]
/// without one.
pub const MAX_RESPONSE_LEN: usize = 2 + 2 + (MAX_VERSION_CODES * VERSION_CODE_LEN);

/// Largest version-code count a requester using `mctp-lib` can decode.
///
/// [`MAX_RESPONSE_LEN`] is sized to this count, and both constructors refuse a
/// longer table.
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

/// What one endpoint reports about itself.
///
/// The binary's `main` fills this in, since only it knows the image's
/// message types and endpoint role.
#[derive(Debug, Clone, Copy)]
pub struct ControlConfig<'a> {
    /// Message types this endpoint answers besides control (DSP0236, Table 19).
    pub supported_msg_types: &'a [u8],
    /// Version codes for the MCTP base specification, 4 bytes per entry.
    pub base_versions: &'a [u8],
    /// Version codes for the control protocol, 4 bytes per entry.
    pub control_versions: &'a [u8],
    /// Endpoint role Get Endpoint ID reports (DSP0236, Table 15).
    pub endpoint_type: EndpointType,
    /// Medium-specific byte Get Endpoint ID returns (DSP0236, Table 15).
    ///
    /// Table 15 requires `0x00` unless the transport binding defines a value.
    pub transport_specific: u8,
}

impl<'a> ControlConfig<'a> {
    /// Build a configuration for a simple endpoint reporting MCTP 1.3.1.
    ///
    /// The [`ControlResponder`] constructor decides whether the EID is static
    /// or dynamic.
    pub const fn simple_endpoint(supported_msg_types: &'a [u8]) -> Self {
        Self {
            supported_msg_types,
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
    /// Returns [`ResponseCode::BadArgument`] for a version table that is not
    /// whole 4-byte codes or holds more than [`MAX_VERSION_CODES`], or for
    /// more than 255 message types.
    pub const fn try_new(config: ControlConfig<'a>) -> Result<Self, MctpError> {
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
    /// Returns [`ResponseCode::BadArgument`] for a `config` [`Self::try_new`]
    /// refuses or an EID no endpoint may hold, or the stack's own error where
    /// the write fails.
    pub fn try_new_with_static_eid<C: MctpClient>(
        stack: &Stack<C>,
        config: ControlConfig<'a>,
        static_eid: u8,
    ) -> Result<Self, MctpError> {
        check_config(&config)?;
        if !is_legal_eid(static_eid) {
            return Err(MctpError::from_code(ResponseCode::BadArgument));
        }
        stack.set_eid(static_eid)?;
        Ok(Self {
            config,
            static_eid: Some(static_eid),
        })
    }

    /// Answer one control request.
    ///
    /// `request` is the payload a receive call produced and `response` takes
    /// the reply, both including the control header. Returns the reply length.
    ///
    /// # Errors
    ///
    /// Every variant of [`ControlError`] means `response` holds no reply to
    /// send, so the caller sends nothing. An unsupported command is not an
    /// error: it answers `ERROR_UNSUPPORTED_CMD`.
    pub fn handle<C: MctpClient>(
        &self,
        stack: &Stack<C>,
        request: &[u8],
        response: &mut [u8],
    ) -> Result<usize, ControlError> {
        let header = MctpControlHeader::decode(request).map_err(|_| ControlError::Malformed)?;
        let body = request
            .get(MctpControlHeader::MCTP_CODEC_MIN_SIZE..)
            .ok_or(ControlError::Malformed)?;

        // See `ControlError::NoReply`: no requester waits for a reply here.
        if !header.request || header.datagram {
            return Err(ControlError::NoReply);
        }

        let reply_header = MctpControlHeader::new(
            /* request= */ false,
            /* datagram= */ false,
            header.instance_id,
            header.command_code,
        );
        let header_len = reply_header
            .encode(response)
            .map_err(|_| ControlError::ResponseTooShort)?;
        let body_buf = response
            .get_mut(header_len..)
            .ok_or(ControlError::ResponseTooShort)?;

        let body_len = self.encode_body(stack, header.command_code, body, body_buf)?;

        Ok(header_len + body_len)
    }

    fn encode_body<C: MctpClient>(
        &self,
        stack: &Stack<C>,
        command: CommandCode,
        body: &[u8],
        out: &mut [u8],
    ) -> Result<usize, ControlError> {
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
    ) -> Result<usize, ControlError> {
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
        .map_err(|_| ControlError::ResponseTooShort)?;

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
    ) -> Result<usize, ControlError> {
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
            .map_err(|_| ControlError::ResponseTooShort)
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

    fn version_support(&self, body: &[u8], out: &mut [u8]) -> Result<usize, ControlError> {
        // DSP0236 Table 18's request carries exactly one byte.
        let &[raw] = body else {
            return encode_error(CompletionCode::ErrorInvalidLength, out);
        };

        let codes = match MctpMessageType::from(raw) {
            MctpMessageType::Mctp => self.config.base_versions,
            MctpMessageType::Control => self.config.control_versions,
            // An upper-layer type asks for that protocol's own version, which
            // this crate holds for none. 0x80 is Table 18's "message type
            // number not supported".
            _ => return encode_error(CompletionCode::CommandSpecific(0x80), out),
        };

        let response = GetMCTPVersionSupportResponse::new(CompletionCode::Success, codes);
        response
            .encode(out)
            .map_err(|_| ControlError::ResponseTooShort)
    }

    fn message_type_support(&self, body: &[u8], out: &mut [u8]) -> Result<usize, ControlError> {
        // DSP0236 Table 19's request carries no data.
        if !body.is_empty() {
            return encode_error(CompletionCode::ErrorInvalidLength, out);
        }
        let response = GetMctpMessageTypeSupportResponse::new(
            CompletionCode::Success,
            self.config.supported_msg_types,
        );
        response
            .encode(out)
            .map_err(|_| ControlError::ResponseTooShort)
    }
}

/// Why [`ControlResponder::handle`] produced no response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
#[non_exhaustive]
pub enum ControlError {
    /// The message was too short to carry a control header.
    Malformed,
    /// The message is a response or a datagram, which draws no reply
    /// (DSP0236, clause 11.5).
    NoReply,
    /// `response` cannot hold the reply; see [`MAX_RESPONSE_LEN`].
    ResponseTooShort,
}

impl fmt::Display for ControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            ControlError::Malformed => "request too short for a control header",
            ControlError::NoReply => "message draws no reply",
            ControlError::ResponseTooShort => "response buffer too short for the reply",
        };
        f.write_str(text)
    }
}

/// Whether an EID may be assigned to an endpoint.
///
/// DSP0236 Table 14 marks the null EID `0x00` and the broadcast EID `0xff`
/// illegal for assignment, and clause 8.2 reserves 1 through 7.
const fn is_legal_eid(eid: u8) -> bool {
    eid > 7 && eid != 0xff
}

/// Refuse a [`ControlConfig`] whose replies `mctp-lib` cannot encode or decode.
///
/// A partial version code or a type list past Table 19's one-byte count
/// would panic in `mctp-lib`'s encoders.
const fn check_config(config: &ControlConfig<'_>) -> Result<(), MctpError> {
    let tables_fit =
        version_table_fits(config.base_versions) && version_table_fits(config.control_versions);
    if tables_fit && config.supported_msg_types.len() <= u8::MAX as usize {
        Ok(())
    } else {
        Err(MctpError::from_code(ResponseCode::BadArgument))
    }
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

fn encode_error(code: CompletionCode, out: &mut [u8]) -> Result<usize, ControlError> {
    *out.first_mut().ok_or(ControlError::ResponseTooShort)? = code.into();
    Ok(1)
}
