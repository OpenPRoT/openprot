// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host tests for the MCTP control responder policy.
//!
//! The responder holds no transport, so these tests need no fabric and no
//! server: request bytes go in and response bytes come out.
//!
//! The endpoint ID lives in the [`Stack`] the responder is handed, so the tests
//! back that stack with `EidClient`.
//!
//! Most expected bytes appear as literals, with the field each carries named in
//! a comment, so a test states what DSP0236 requires rather than what the
//! encoder computes. Clause, table, and figure numbers refer to DSP0236 1.3.1.

use core::cell::Cell;

use mctp_lib::mctp_control::CommandCode;
use mctp_lib::mctp_control::MctpCodec;
use mctp_lib::mctp_control::MctpControlHeader;

use openprot_mctp_api::Handle;
use openprot_mctp_api::MctpClient;
use openprot_mctp_api::MctpError;
use openprot_mctp_api::RecvMetadata;
use openprot_mctp_api::ResponseCode;
use openprot_mctp_api::Stack;

use openprot_mctp_control::ControlConfig;
use openprot_mctp_control::ControlError;
use openprot_mctp_control::ControlResponder;
use openprot_mctp_control::MAX_RESPONSE_LEN;

/// Message types the tested endpoint reports as supported, control excluded.
static SUPPORTED_MSG_TYPES: [u8; 2] = [0x01, 0x05];

/// The EID the stack holds until a request assigns another.
const LOCAL_EID: u8 = 0x1d;

/// The EID a Set Endpoint ID request assigns.
const ASSIGNED_EID: u8 = 0x2a;

const STATIC_EID: u8 = 0x0c;

/// An instance ID filling all 5 bits the field carries (DSP0236, Figure 20).
const INSTANCE_ID: u8 = 0x1f;

/// The version codes DSP0236 1.3.1 requires in both version tables.
///
/// Clauses 12.6.2 and 12.6.3 give the base specification and the control
/// protocol the same four entries. They are written out here rather than taken
/// from the crate, so a typo in the crate's table fails a test.
const SPEC_VERSIONS_1_3_1: [u8; 16] = [
    0xf1, 0xf0, 0xff, 0x00, // 1.0, any update, no alpha
    0xf1, 0xf1, 0xff, 0x00, // 1.1, any update, no alpha
    0xf1, 0xf2, 0xff, 0x00, // 1.2, any update, no alpha
    0xf1, 0xf3, 0xf1, 0x00, // 1.3.1, no alpha
];

/// Stores the EID, and fails the test on any transport call.
struct EidClient {
    eid: Cell<u8>,
    refuse_set: bool,
}

impl EidClient {
    const fn new(eid: u8) -> Self {
        Self {
            eid: Cell::new(eid),
            refuse_set: false,
        }
    }

    /// A client whose `set_eid` fails, standing in for a stack that refuses.
    const fn refusing(eid: u8) -> Self {
        Self {
            eid: Cell::new(eid),
            refuse_set: true,
        }
    }
}

impl MctpClient for EidClient {
    fn get_eid(&self) -> u8 {
        self.eid.get()
    }

    fn set_eid(&self, eid: u8) -> Result<(), MctpError> {
        if self.refuse_set {
            return Err(MctpError::from_code(ResponseCode::InternalError));
        }
        self.eid.set(eid);
        Ok(())
    }

    fn req(&self, _eid: u8) -> Result<Handle, MctpError> {
        unreachable!("the responder opens no request channel")
    }

    fn listener(&self, _msg_type: u8) -> Result<Handle, MctpError> {
        unreachable!("the caller registers the listener, not the responder")
    }

    fn recv(
        &self,
        _handle: Handle,
        _timeout_millis: u32,
        _buf: &mut [u8],
    ) -> Result<RecvMetadata, MctpError> {
        unreachable!("the caller receives, not the responder")
    }

    fn send(
        &self,
        _handle: Option<Handle>,
        _msg_type: u8,
        _eid: Option<u8>,
        _tag: Option<u8>,
        _integrity_check: bool,
        _buf: &[u8],
    ) -> Result<u8, MctpError> {
        unreachable!("the caller sends, not the responder")
    }

    fn drop_handle(&self, _handle: Handle) {
        unreachable!("the responder holds no handle")
    }
}

fn stack_new() -> Stack<EidClient> {
    Stack::new(EidClient::new(LOCAL_EID))
}

/// Build the dynamic-endpoint responder most tests drive.
fn responder_new() -> ControlResponder<'static> {
    ControlResponder::try_new(ControlConfig::simple_endpoint(&SUPPORTED_MSG_TYPES))
        .expect("the simple-endpoint config is valid")
}

/// Build a static responder over `stack`, which the constructor seeds.
fn static_responder_new<C: MctpClient>(stack: &Stack<C>) -> ControlResponder<'static> {
    ControlResponder::try_new_with_static_eid(
        stack,
        ControlConfig::simple_endpoint(&SUPPORTED_MSG_TYPES),
        STATIC_EID,
    )
    .expect("the stack accepts the configured EID")
}

/// Encode one control message: the two header bytes, then the body bytes.
fn message_encode(request: bool, command: CommandCode, body: &[u8]) -> Vec<u8> {
    let header = MctpControlHeader::new(request, /* datagram= */ false, INSTANCE_ID, command);
    let mut bytes = vec![0u8; MctpControlHeader::MCTP_CODEC_MIN_SIZE];
    header
        .encode(&mut bytes)
        .expect("two bytes hold one control header");
    bytes.extend_from_slice(body);
    bytes
}

fn request_encode(command: CommandCode, body: &[u8]) -> Vec<u8> {
    message_encode(/* request= */ true, command, body)
}

#[test]
fn get_endpoint_id_reports_the_eid_the_stack_holds() {
    let responder = responder_new();
    let stack = stack_new();
    let request = request_encode(CommandCode::GetEndpointID, &[]);
    let mut response = [0u8; MAX_RESPONSE_LEN];

    let len = responder
        .handle(&stack, &request, &mut response)
        .expect("a well-formed request produces a response");

    // Byte 0 is rq=0, d=0, and the request's instance ID, and byte 1 is the
    // request's command code, Get Endpoint ID: DSP0236 Table 10 has a response
    // echo both. Then Success, the reported EID, 0x00 for a simple endpoint
    // with a dynamic EID, then 0x00 for Table 15's medium-specific byte.
    let expected = [INSTANCE_ID, 0x02, 0x00, LOCAL_EID, 0x00, 0x00];
    assert_eq!(len, 6);
    assert_eq!(response.get(..len), Some(expected.as_slice()));
}

#[test]
fn set_endpoint_id_accepts_a_legal_eid_and_writes_the_stack() {
    let responder = responder_new();

    // Clause 8.2 reserves 0x01 through 0x07 and Table 14 makes 0xff illegal,
    // so 0x08 and 0xfe are the two ends of the assignable range.
    for eid in [0x08u8, ASSIGNED_EID, 0xfe] {
        let stack = stack_new();
        // Operation 00b is Set EID, and byte 2 carries the value to assign.
        let request = request_encode(CommandCode::SetEndpointID, &[0x00, eid]);
        let mut response = [0u8; MAX_RESPONSE_LEN];

        let len = responder
            .handle(&stack, &request, &mut response)
            .expect("a well-formed request produces a response");

        // Header, then Success, accepted with no pool, the EID from the
        // request (Table 14), then a pool size of 0.
        let expected = [INSTANCE_ID, 0x01, 0x00, 0x00, eid, 0x00];
        assert_eq!(len, 6, "{eid:#04x} is accepted");
        assert_eq!(response.get(..len), Some(expected.as_slice()), "{eid:#04x}");
        assert_eq!(stack.get_eid(), eid, "{eid:#04x} lands in the stack");
    }
}

/// The response claims no assignment that did not happen.
#[test]
fn set_endpoint_id_answers_an_error_when_the_stack_refuses() {
    let responder = responder_new();
    let stack = Stack::new(EidClient::refusing(LOCAL_EID));
    let request = request_encode(CommandCode::SetEndpointID, &[0x00, ASSIGNED_EID]);
    let mut response = [0u8; MAX_RESPONSE_LEN];

    let len = responder
        .handle(&stack, &request, &mut response)
        .expect("a well-formed request produces a response");

    // Header, then ERROR alone as the body: Table 13's generic failure, since
    // the refusal is the endpoint's own and not the request's.
    let expected = [INSTANCE_ID, 0x01, 0x01];
    assert_eq!(len, 3);
    assert_eq!(response.get(..len), Some(expected.as_slice()));
    assert_eq!(stack.get_eid(), LOCAL_EID);
}

#[test]
fn force_eid_answers_the_same_as_set_eid_on_one_bus() {
    let responder = responder_new();
    let stack = stack_new();
    let forcing_stack = stack_new();
    // Operation 00b is Set EID and 01b is Force EID. Table 14 makes the two
    // equivalent for an endpoint reached through one bus.
    let set = request_encode(CommandCode::SetEndpointID, &[0x00, ASSIGNED_EID]);
    let force = request_encode(CommandCode::SetEndpointID, &[0x01, ASSIGNED_EID]);
    let mut set_response = [0u8; MAX_RESPONSE_LEN];
    let mut force_response = [0u8; MAX_RESPONSE_LEN];

    // Each operation drives its own stack, so the first assignment cannot
    // change what the second one starts from.
    let set_len = responder
        .handle(&stack, &set, &mut set_response)
        .expect("a well-formed request produces a response");
    let force_len = responder
        .handle(&forcing_stack, &force, &mut force_response)
        .expect("a well-formed request produces a response");

    assert_eq!(set_len, force_len);
    assert_eq!(set_response.get(..set_len), force_response.get(..force_len));
    assert_eq!(forcing_stack.get_eid(), ASSIGNED_EID);
}

#[test]
fn set_endpoint_id_refuses_an_eid_no_endpoint_may_hold() {
    let responder = responder_new();

    // Table 14 marks the null EID 0x00 and the broadcast EID 0xff illegal for
    // assignment, and clause 8.2 reserves 0x01 through 0x07.
    for eid in [0x00u8, 0x01, 0x07, 0xff] {
        let stack = stack_new();
        let request = request_encode(CommandCode::SetEndpointID, &[0x00, eid]);
        let mut response = [0u8; MAX_RESPONSE_LEN];

        let len = responder
            .handle(&stack, &request, &mut response)
            .expect("a well-formed request produces a response");

        // Header, then ERROR_INVALID_DATA alone as the body.
        let expected = [INSTANCE_ID, 0x01, 0x02];
        assert_eq!(len, 3, "{eid:#04x} answers one code");
        assert_eq!(response.get(..len), Some(expected.as_slice()), "{eid:#04x}");
        assert_eq!(stack.get_eid(), LOCAL_EID, "{eid:#04x} takes no write");
    }
}

/// Table 13 defines ERROR_INVALID_LENGTH for a body "larger or smaller than
/// expected", and keeps ERROR_INVALID_DATA for an illegal value.
#[test]
fn set_endpoint_id_without_its_eid_byte_answers_invalid_length() {
    let responder = responder_new();

    // An empty body, then operations 00b and 01b with byte 2 left out.
    for body in [[].as_slice(), [0x00u8].as_slice(), [0x01u8].as_slice()] {
        let stack = stack_new();
        let request = request_encode(CommandCode::SetEndpointID, body);
        let mut response = [0u8; MAX_RESPONSE_LEN];

        let len = responder
            .handle(&stack, &request, &mut response)
            .expect("a header-only or short request still produces a response");

        // Header, then ERROR_INVALID_LENGTH alone as the body.
        let expected = [INSTANCE_ID, 0x01, 0x03];
        assert_eq!(len, 3, "body {body:02x?}");
        assert_eq!(
            response.get(..len),
            Some(expected.as_slice()),
            "{body:02x?}"
        );
        assert_eq!(stack.get_eid(), LOCAL_EID, "{body:02x?} takes no write");
    }
}

#[test]
fn reset_eid_and_set_discovered_flag_answer_invalid_data() {
    let responder = responder_new();

    // Table 14 has the endpoint ignore byte 2 for both operations, and this
    // crate also accepts a body that leaves it out, so each runs both ways.
    // Each answers ERROR_INVALID_DATA, where a decode reading byte 2 before
    // the operation would answer the one-byte body with ERROR_INVALID_LENGTH.
    for operation in [0x02u8, 0x03] {
        for body in [[operation, 0x00].as_slice(), [operation].as_slice()] {
            let stack = stack_new();
            let request = request_encode(CommandCode::SetEndpointID, body);
            let mut response = [0u8; MAX_RESPONSE_LEN];

            let len = responder
                .handle(&stack, &request, &mut response)
                .expect("the operation decodes and answers a completion code");

            // Header, then ERROR_INVALID_DATA, which Table 14 requires of
            // Reset EID on an endpoint without a static EID and of Set
            // Discovered Flag on a binding without a Discovered flag.
            let expected = [INSTANCE_ID, 0x01, 0x02];
            assert_eq!(
                len,
                3,
                "operation {operation:#04x}, body of {} bytes",
                body.len()
            );
            assert_eq!(
                response.get(..len),
                Some(expected.as_slice()),
                "operation {operation:#04x}, body of {} bytes",
                body.len()
            );
            assert_eq!(
                stack.get_eid(),
                LOCAL_EID,
                "operation {operation:#04x}, body of {} bytes takes no write",
                body.len()
            );
        }
    }
}

#[test]
fn get_message_type_support_lists_the_configured_types() {
    let responder = responder_new();
    let stack = stack_new();
    let request = request_encode(CommandCode::GetMessageTypeSupport, &[]);
    let mut response = [0u8; MAX_RESPONSE_LEN];

    let len = responder
        .handle(&stack, &request, &mut response)
        .expect("a well-formed request produces a response");

    // Header, then Success, a count of 2, then PLDM and SPDM. Table 19 counts
    // the types in addition to control itself, so 0x00 is absent.
    let expected = [INSTANCE_ID, 0x05, 0x00, 0x02, 0x01, 0x05];
    assert_eq!(len, 6);
    assert_eq!(response.get(..len), Some(expected.as_slice()));
}

#[test]
fn get_version_support_reports_the_base_versions_for_message_type_ff() {
    let responder = responder_new();
    let stack = stack_new();
    // 0xff asks for the MCTP base specification version.
    let request = request_encode(CommandCode::GetMCTPVersionSupport, &[0xff]);
    let mut response = [0u8; MAX_RESPONSE_LEN];

    let len = responder
        .handle(&stack, &request, &mut response)
        .expect("a well-formed request produces a response");

    // After the header: Success, an entry count of 4, then 16 version bytes.
    assert_eq!(len, 20);
    assert_eq!(response.get(2).copied(), Some(0x00));
    assert_eq!(response.get(3).copied(), Some(0x04));
    assert_eq!(response.get(4..len), Some(SPEC_VERSIONS_1_3_1.as_slice()));
}

/// Under 1.3.1 the control protocol's entries match the base specification's,
/// so this case cannot tell the two config fields apart;
/// `get_version_support_reads_each_table_from_its_own_field` does.
#[test]
fn get_version_support_reports_the_control_versions_for_message_type_00() {
    let responder = responder_new();
    let stack = stack_new();
    let request = request_encode(CommandCode::GetMCTPVersionSupport, &[0x00]);
    let mut response = [0u8; MAX_RESPONSE_LEN];

    let len = responder
        .handle(&stack, &request, &mut response)
        .expect("a well-formed request produces a response");

    assert_eq!(len, 20);
    assert_eq!(response.get(2).copied(), Some(0x00));
    assert_eq!(response.get(3).copied(), Some(0x04));
    assert_eq!(response.get(4..len), Some(SPEC_VERSIONS_1_3_1.as_slice()));
}

#[test]
fn get_version_support_reads_each_table_from_its_own_field() {
    static BASE_ONLY: [u8; 4] = [0xf1, 0xf3, 0xf1, 0x00];
    static CONTROL_ONLY: [u8; 4] = [0xf1, 0xf2, 0xff, 0x00];
    let config = ControlConfig {
        base_versions: &BASE_ONLY,
        control_versions: &CONTROL_ONLY,
        ..ControlConfig::simple_endpoint(&SUPPORTED_MSG_TYPES)
    };
    let responder = ControlResponder::try_new(config).expect("one-code tables are valid");
    let stack = stack_new();

    // 0xff reads base_versions and 0x00 reads control_versions. Each reply is
    // the header, Success, an entry count of 1, then that table's one entry.
    for (msg_type, expected) in [
        (
            0xffu8,
            [INSTANCE_ID, 0x04, 0x00, 0x01, 0xf1, 0xf3, 0xf1, 0x00],
        ),
        (
            0x00,
            [INSTANCE_ID, 0x04, 0x00, 0x01, 0xf1, 0xf2, 0xff, 0x00],
        ),
    ] {
        let request = request_encode(CommandCode::GetMCTPVersionSupport, &[msg_type]);
        let mut response = [0u8; MAX_RESPONSE_LEN];

        let len = responder
            .handle(&stack, &request, &mut response)
            .expect("a well-formed request produces a response");

        assert_eq!(len, 8, "message type {msg_type:#04x}");
        assert_eq!(
            response.get(..len),
            Some(expected.as_slice()),
            "message type {msg_type:#04x} answers from its own table"
        );
    }
}

/// Table 18's request carries one byte.
#[test]
fn get_version_support_without_a_message_type_answers_invalid_length() {
    let responder = responder_new();
    let stack = stack_new();
    let request = request_encode(CommandCode::GetMCTPVersionSupport, &[]);
    let mut response = [0u8; MAX_RESPONSE_LEN];

    let len = responder
        .handle(&stack, &request, &mut response)
        .expect("a header-only request still produces a response");

    // Header, then ERROR_INVALID_LENGTH alone as the body.
    let expected = [INSTANCE_ID, 0x04, 0x03];
    assert_eq!(len, 3);
    assert_eq!(response.get(..len), Some(expected.as_slice()));
}

#[test]
fn a_body_longer_than_dsp0236_defines_answers_invalid_length() {
    let responder = responder_new();

    // Tables 15 and 19 define no request data, Table 18 one byte, and Table 14
    // two bytes, so each body carries one byte past its command's length.
    for (case, command, body, expected) in [
        (
            "Get Endpoint ID",
            CommandCode::GetEndpointID,
            [0x00u8].as_slice(),
            [INSTANCE_ID, 0x02, 0x03],
        ),
        (
            "Get Message Type Support",
            CommandCode::GetMessageTypeSupport,
            [0x00u8].as_slice(),
            [INSTANCE_ID, 0x05, 0x03],
        ),
        (
            "Get MCTP Version Support",
            CommandCode::GetMCTPVersionSupport,
            [0xffu8, 0x00].as_slice(),
            [INSTANCE_ID, 0x04, 0x03],
        ),
        (
            "Set Endpoint ID",
            CommandCode::SetEndpointID,
            [0x00u8, ASSIGNED_EID, 0x00].as_slice(),
            [INSTANCE_ID, 0x01, 0x03],
        ),
    ] {
        let stack = stack_new();
        let request = request_encode(command, body);
        let mut response = [0u8; MAX_RESPONSE_LEN];

        let len = responder
            .handle(&stack, &request, &mut response)
            .expect("an over-long request still produces a response");

        assert_eq!(len, 3, "{case}");
        assert_eq!(response.get(..len), Some(expected.as_slice()), "{case}");
        // Set Endpoint ID is the one case that could write the stack.
        assert_eq!(stack.get_eid(), LOCAL_EID, "{case} takes no write");
    }
}

#[test]
fn an_unsupported_command_answers_unsupported_cmd_under_its_own_code() {
    let responder = responder_new();
    let stack = stack_new();
    // 0x06 is Get Vendor Defined Message Support, which this crate defers.
    let request = request_encode(CommandCode::GetVendorDefinedMessageSupport, &[0x00]);
    let mut response = [0u8; MAX_RESPONSE_LEN];

    let len = responder
        .handle(&stack, &request, &mut response)
        .expect("a well-formed request produces a response");

    // Header carrying the request's own command code, then
    // ERROR_UNSUPPORTED_CMD.
    let expected = [INSTANCE_ID, 0x06, 0x05];
    assert_eq!(len, 3);
    assert_eq!(response.get(..len), Some(expected.as_slice()));
}

#[test]
fn a_request_too_short_for_a_header_is_malformed() {
    let responder = responder_new();
    let stack = stack_new();
    let mut response = [0u8; MAX_RESPONSE_LEN];

    // One byte cannot carry the two-byte control header.
    assert_eq!(
        responder.handle(&stack, &[0xbf], &mut response),
        Err(ControlError::Malformed)
    );
}

#[test]
fn a_response_buffer_too_short_for_the_reply_is_rejected() {
    let responder = responder_new();
    let stack = stack_new();
    let request = request_encode(CommandCode::GetMCTPVersionSupport, &[0xff]);

    // Two bytes hold the header and nothing else.
    let mut header_short = [0u8; 2];
    assert_eq!(
        responder.handle(&stack, &request, &mut header_short),
        Err(ControlError::ResponseTooShort)
    );
}

/// The caller sends nothing on an error, so the bus owner records no
/// assignment. Moving the stack anyway would put the endpoint at an EID the
/// bus owner does not know.
#[test]
fn set_endpoint_id_without_room_for_its_reply_keeps_the_eid() {
    let responder = responder_new();
    let stack = stack_new();
    let request = request_encode(CommandCode::SetEndpointID, &[0x00, ASSIGNED_EID]);

    // Five bytes hold the two-byte header but not the four-byte body.
    let mut body_short = [0u8; 5];
    assert_eq!(
        responder.handle(&stack, &request, &mut body_short),
        Err(ControlError::ResponseTooShort)
    );
    assert_eq!(stack.get_eid(), LOCAL_EID);
}

#[test]
fn a_response_message_draws_no_reply() {
    let responder = responder_new();
    let stack = stack_new();
    // DSP0236 Table 11: rq=0 is a response, not a request. Answering it would
    // put a message on the bus that no requester is waiting for.
    let message = message_encode(
        /* request= */ false,
        CommandCode::SetEndpointID,
        &[0x00, ASSIGNED_EID],
    );
    let mut response = [0u8; MAX_RESPONSE_LEN];

    assert_eq!(
        responder.handle(&stack, &message, &mut response),
        Err(ControlError::NoReply)
    );
    assert_eq!(stack.get_eid(), LOCAL_EID);
}

/// DSP0236 Table 11 states that for a datagram "responses are neither expected
/// nor tracked by Instance ID at the requester".
#[test]
fn a_datagram_draws_no_reply() {
    let responder = responder_new();
    let stack = stack_new();
    // With D=0 this Get Endpoint ID draws a reply, so the D bit alone
    // separates the two.
    let header = MctpControlHeader::new(
        /* request= */ true,
        /* datagram= */ true,
        INSTANCE_ID,
        CommandCode::GetEndpointID,
    );
    let mut message = vec![0u8; MctpControlHeader::MCTP_CODEC_MIN_SIZE];
    header
        .encode(&mut message)
        .expect("two bytes hold one control header");
    let mut response = [0u8; MAX_RESPONSE_LEN];

    assert_eq!(
        responder.handle(&stack, &message, &mut response),
        Err(ControlError::NoReply)
    );
}

/// DSP0236 Table 15 defines EID type `10b` as "static EID supported. Present
/// EID matches static EID".
#[test]
fn get_endpoint_id_reports_a_static_eid_that_still_matches() {
    let stack = Stack::new(EidClient::new(0x00));
    let responder = static_responder_new(&stack);
    let request = request_encode(CommandCode::GetEndpointID, &[]);
    let mut response = [0u8; MAX_RESPONSE_LEN];

    let len = responder
        .handle(&stack, &request, &mut response)
        .expect("a well-formed request produces a response");

    let expected = [INSTANCE_ID, 0x02, 0x00, STATIC_EID, 0x02, 0x00];
    assert_eq!(len, 6);
    assert_eq!(response.get(..len), Some(expected.as_slice()));
}

/// Table 15 defines EID type `11b` as "static EID supported. Present EID does
/// not match static EID".
#[test]
fn get_endpoint_id_reports_a_static_eid_the_bus_owner_moved() {
    let stack = Stack::new(EidClient::new(0x00));
    let responder = static_responder_new(&stack);
    let assign = request_encode(CommandCode::SetEndpointID, &[0x00, ASSIGNED_EID]);
    let query = request_encode(CommandCode::GetEndpointID, &[]);
    let mut response = [0u8; MAX_RESPONSE_LEN];

    responder
        .handle(&stack, &assign, &mut response)
        .expect("a static endpoint accepts an assignment");
    let len = responder
        .handle(&stack, &query, &mut response)
        .expect("a well-formed request produces a response");

    let expected = [INSTANCE_ID, 0x02, 0x00, ASSIGNED_EID, 0x03, 0x00];
    assert_eq!(len, 6);
    assert_eq!(response.get(..len), Some(expected.as_slice()));
}

/// Table 14: "If static EIDs are supported, the endpoint shall restore the EID
/// to the statically configured EID value."
#[test]
fn reset_eid_restores_the_configured_eid() {
    // Table 14 has the endpoint ignore byte 2 here, and this crate also
    // accepts a body that leaves byte 2 out. The static responder answers
    // Success, so the two-byte case, whose byte 2 is the illegal EID 0x00,
    // fails if decode_set_eid validates byte 2 as an EID, and the one-byte
    // case fails if decode_set_eid reads byte 2 before the operation.
    for body in [[0x02u8, 0x00].as_slice(), [0x02u8].as_slice()] {
        let stack = Stack::new(EidClient::new(0x00));
        let responder = static_responder_new(&stack);
        let assign = request_encode(CommandCode::SetEndpointID, &[0x00, ASSIGNED_EID]);
        let reset = request_encode(CommandCode::SetEndpointID, body);
        let mut response = [0u8; MAX_RESPONSE_LEN];

        // Construction already wrote STATIC_EID, so the stack moves away first:
        // only a Reset EID that writes the stack then brings it back.
        responder
            .handle(&stack, &assign, &mut response)
            .expect("a static endpoint accepts an assignment");
        assert_eq!(
            stack.get_eid(),
            ASSIGNED_EID,
            "the assignment moved the stack"
        );

        let len = responder
            .handle(&stack, &reset, &mut response)
            .expect("a static endpoint supports Reset EID");

        // Header, then Success, accepted with no pool, the restored EID, then a
        // pool size of 0.
        let expected = [INSTANCE_ID, 0x01, 0x00, 0x00, STATIC_EID, 0x00];
        assert_eq!(len, 6, "body of {} bytes", body.len());
        assert_eq!(
            response.get(..len),
            Some(expected.as_slice()),
            "body of {} bytes",
            body.len()
        );
        assert_eq!(stack.get_eid(), STATIC_EID, "body of {} bytes", body.len());
    }
}

/// Table 14 makes 0x00 and 0xff illegal, and clause 8.2 reserves 0x01 through
/// 0x07.
#[test]
fn an_illegal_static_eid_is_refused_without_writing_the_stack() {
    for eid in [0x00u8, 0x01, 0x07, 0xff] {
        let stack = stack_new();
        let config = ControlConfig::simple_endpoint(&SUPPORTED_MSG_TYPES);

        let error = ControlResponder::try_new_with_static_eid(&stack, config, eid)
            .err()
            .unwrap_or_else(|| panic!("{eid:#04x} must be refused"));

        assert_eq!(error.code, ResponseCode::BadArgument, "{eid:#04x}");
        assert_eq!(stack.get_eid(), LOCAL_EID, "{eid:#04x} writes nothing");
    }
}

/// The `mctp-lib` encoders panic on a partial version code or on more than
/// 255 message types. A table of more than 8 codes encodes, but a `mctp-lib`
/// requester rejects that reply, since its decoder refuses a count above 8.
#[test]
fn a_config_mctp_lib_cannot_encode_is_refused() {
    static SIX_BYTES: [u8; 6] = [0xf1; 6];
    static NINE_CODES: [u8; 36] = [0xf1; 36];
    static TYPES_256: [u8; 256] = [0x01; 256];
    let valid = ControlConfig::simple_endpoint(&SUPPORTED_MSG_TYPES);

    for (case, config) in [
        (
            "a 6-byte base table",
            ControlConfig {
                base_versions: &SIX_BYTES,
                ..valid
            },
        ),
        (
            "9 control codes",
            ControlConfig {
                control_versions: &NINE_CODES,
                ..valid
            },
        ),
        (
            "256 message types",
            ControlConfig {
                supported_msg_types: &TYPES_256,
                ..valid
            },
        ),
    ] {
        let dynamic = ControlResponder::try_new(config).expect_err(case);
        assert_eq!(dynamic.code, ResponseCode::BadArgument, "dynamic, {case}");

        let stack = stack_new();
        let error =
            ControlResponder::try_new_with_static_eid(&stack, config, STATIC_EID).expect_err(case);
        assert_eq!(error.code, ResponseCode::BadArgument, "static, {case}");
        assert_eq!(stack.get_eid(), LOCAL_EID, "{case} writes nothing");
    }
}

/// The cases include both ends of the assignable range, 0x08 and 0xfe. The
/// hardware image configures EID 8, so a constructor that refused 0x08 would
/// leave the hardware image without a responder.
#[test]
fn building_a_static_responder_seeds_the_stack() {
    for eid in [0x08u8, STATIC_EID, 0xfe] {
        let stack = Stack::new(EidClient::new(0x00));
        let config = ControlConfig::simple_endpoint(&SUPPORTED_MSG_TYPES);

        let built = ControlResponder::try_new_with_static_eid(&stack, config, eid);

        assert!(built.is_ok(), "{eid:#04x} must be accepted");
        assert_eq!(stack.get_eid(), eid, "{eid:#04x} seeds the stack");
    }
}

/// The seeding write's own error passes through, since `BadArgument` stays
/// reserved for an illegal EID.
#[test]
fn a_static_responder_reports_a_refused_seeding_write() {
    let stack = Stack::new(EidClient::refusing(0x00));
    let config = ControlConfig::simple_endpoint(&SUPPORTED_MSG_TYPES);

    let error = ControlResponder::try_new_with_static_eid(&stack, config, STATIC_EID)
        .expect_err("a refused seeding write must be reported");

    // EidClient::refusing answers InternalError.
    assert_eq!(error.code, ResponseCode::InternalError);
    assert_eq!(stack.get_eid(), 0x00, "the refused write leaves the stack");
}
