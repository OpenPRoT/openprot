// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! OCP-EAT COSE_Sign1 token assembly.
//!
//! Token structure (CBOR diagnostic notation):
//!
//! ```text
//! 18(                          ; COSE_Sign1
//!   [
//!     << { 1: -35 } >>,        ; protected header (alg only)
//!     { 33: [cert-chain...] }, ; unprotected header (x5chain per OCP profile)
//!     << 55799(61({ ...CWT claims... })) >>,  ; payload (self-described EAT CWT)
//!     h'...'                   ; ES384 signature
//!   ]
//! )
//! ```
//!
//! Claim key numbers follow RFC 9711 and the OCP-EAT profile.
//! Claim order follows CBOR deterministic encoding (RFC 8949 §4.2.1):
//! keys sorted by bytewise lexicographic order of their CBOR encodings.

use heapless::Vec;
use minicbor::encode::write::EndOfSlice;
use minicbor::Encoder;

use openprot_attest_api::consts::{MAX_CERT_SIZE, MAX_CHAIN_LEN, MAX_TOKEN_SIZE};
use openprot_attest_api::{AttestConfig, AttestError, DigestAlgorithm, HwSigner, Measurement};

use crate::cert_ueid::UEID_LEN;

// Registered EAT claim keys (OCP-EAT profile / RFC 9711 / RFC 8392).
// Written in CBOR deterministic order (RFC 8949 §4.2.1): sorted by
// bytewise lexicographic order of each key's CBOR encoding.
//   1-byte key  (0x0a):     nonce
//   3-byte keys (0x1901xx): ueid, oemid, hwmodel, dbgstat, eat_profile, measurements
const CLAIM_NONCE: i64 = 10;
const CLAIM_UEID: i64 = 256;
const CLAIM_OEMID: i64 = 258;
const CLAIM_HWMODEL: i64 = 259;
const CLAIM_DBGSTAT: i64 = 263;
const CLAIM_EAT_PROFILE: i64 = 265;
const CLAIM_MEASUREMENTS: i64 = 273;

const ALG_ES384: i64 = -35;
const HDR_X5CHAIN: i64 = 33;

// OID 1.3.6.1.4.1.42623.1.3 encoded as raw OID content bytes (~oid per OCP profile CDDL).
const OCP_PROFILE_OID: [u8; 10] = [0x2B, 0x06, 0x01, 0x04, 0x01, 0x82, 0xCC, 0x7F, 0x01, 0x03];

// Fixed scratch buffers used during token construction.
const SCRATCH: usize = MAX_TOKEN_SIZE;
// Protected header contains only {1: -35} ≈ 5 bytes; small fixed buffer suffices.
const PHDR_SCRATCH: usize = 16;

/// Writer over a fixed `[u8]` slice; tracks how many bytes have been written.
type BufWriter<'a> = minicbor::encode::write::Cursor<&'a mut [u8]>;

fn cbor_err(e: minicbor::encode::Error<EndOfSlice>) -> AttestError {
    if e.is_write() {
        AttestError::BufferFull
    } else {
        AttestError::Cbor
    }
}

/// Nonce length bounds per RFC 9711 §4.3.4.3 and OCP-EAT profile.
const MIN_NONCE_LEN: usize = 8;
const MAX_NONCE_LEN: usize = 64;

/// Build and sign a complete OCP-EAT token into `out`.
pub(crate) fn build(
    config: &AttestConfig,
    signer: &dyn HwSigner,
    ueid: &[u8; UEID_LEN],
    measurements: &[Measurement],
    nonce: &[u8],
    out: &mut Vec<u8, MAX_TOKEN_SIZE>,
) -> Result<(), AttestError> {
    if nonce.len() < MIN_NONCE_LEN || nonce.len() > MAX_NONCE_LEN {
        return Err(AttestError::Caliptra(
            "nonce must be 8–64 bytes (RFC 9711 §4.3.4.3, OCP-EAT profile)",
        ));
    }

    // Fetch cert chain into a stack buffer.
    let mut chain: Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN> = Vec::new();
    signer.cert_chain_der(&mut chain)?;

    // ── Encode protected header ────────────────────────────────────────────
    // Per OCP-EAT profile: protected header contains only the algorithm ID.
    // x5chain goes in the unprotected header (not covered by the signature).
    // Must be encoded before signing so it can be included in Sig_Structure.
    let mut phdr_scratch = [0u8; PHDR_SCRATCH];
    let phdr_len = (|| -> Result<usize, minicbor::encode::Error<EndOfSlice>> {
        let mut w = BufWriter::new(&mut phdr_scratch[..]);
        let mut e = Encoder::new(&mut w);
        e.map(1)?;
        e.i64(1)?;
        e.i64(ALG_ES384)?;
        Ok(w.position())
    })()
    .map_err(cbor_err)?;
    let phdr_bytes = &phdr_scratch[..phdr_len];

    // ── Encode CWT claims map ──────────────────────────────────────────────
    // Claims are written in CBOR deterministic order (RFC 8949 §4.2.1).
    let mut payload_scratch = [0u8; SCRATCH];
    let payload_len = (|| -> Result<usize, minicbor::encode::Error<EndOfSlice>> {
        // Fixed claims (7): nonce, ueid, oemid, hwmodel, dbgstat,
        // eat_profile, measurements. Update on any change below.
        const FIXED_CLAIMS: usize = 7;
        let n_claims = FIXED_CLAIMS;
        let mut w = BufWriter::new(&mut payload_scratch[..]);
        let mut e = Encoder::new(&mut w);
        // OCP-EAT profile requires tag(55799) wrapping the CWT tag(61).
        e.tag(minicbor::data::Tag::new(55799))?;
        e.tag(minicbor::data::Tag::new(61))?;
        e.map(n_claims as u64)?;

        // ── 1-byte key (10 = nonce) ──────────────────────────────────────
        e.i64(CLAIM_NONCE)?;
        e.bytes(nonce)?;

        // ── 3-byte keys (sorted: 256, 258, 259, 263, 265, 273) ───────────
        e.i64(CLAIM_UEID)?;
        e.bytes(ueid)?;

        e.i64(CLAIM_OEMID)?;
        e.bytes(&config.oemid.0)?;

        e.i64(CLAIM_HWMODEL)?;
        e.str(&config.hw_model)?;

        // dbgstat = 3 (disabled)
        e.i64(CLAIM_DBGSTAT)?;
        e.i64(3)?;

        // eat_profile OID 1.3.6.1.4.1.42623.1.3 (raw OID bytes, ~oid per OCP CDDL)
        e.i64(CLAIM_EAT_PROFILE)?;
        e.bytes(&OCP_PROFILE_OID)?;

        // measurements array (key 273 per OCP-EAT profile)
        e.i64(CLAIM_MEASUREMENTS)?;
        e.array(measurements.len() as u64)?;
        for m in measurements {
            let alg: i64 = match m.digest_alg {
                DigestAlgorithm::Sha384 => -43,
                DigestAlgorithm::Sha512 => -44,
            };
            e.array(3)?;
            e.str(&m.component)?;
            e.i64(alg)?;
            e.bytes(&m.digest)?;
        }

        Ok(w.position())
    })()
    .map_err(cbor_err)?;
    let payload_bytes = &payload_scratch[..payload_len];

    // ── Sign ──────────────────────────────────────────────────────────────
    // RFC 9052 §4.4: signature input is Sig_Structure =
    //   ["Signature1", phdr_bstr, h'', payload_bstr]
    let sig = {
        let mut sig_scratch = [0u8; SCRATCH];
        let sig_input_len = (|| -> Result<usize, minicbor::encode::Error<EndOfSlice>> {
            let mut w = BufWriter::new(&mut sig_scratch[..]);
            let mut e = Encoder::new(&mut w);
            e.array(4)?;
            e.str("Signature1")?;
            e.bytes(phdr_bytes)?;
            e.bytes(b"")?; // aad = h''
            e.bytes(payload_bytes)?;
            Ok(w.position())
        })()
        .map_err(cbor_err)?;
        signer.sign(&sig_scratch[..sig_input_len])?
    };

    // ── Assemble COSE_Sign1 ────────────────────────────────────────────────
    // Unprotected header carries x5chain (not signed, per OCP-EAT profile).
    let mut cose_scratch = [0u8; SCRATCH];
    let cose_len = (|| -> Result<usize, minicbor::encode::Error<EndOfSlice>> {
        let mut w = BufWriter::new(&mut cose_scratch[..]);
        let mut e = Encoder::new(&mut w);
        e.tag(minicbor::data::Tag::new(18))?;
        e.array(4)?;
        e.bytes(phdr_bytes)?;
        // Unprotected header: {33: [cert0, cert1, ...]}
        e.map(1)?;
        e.i64(HDR_X5CHAIN)?;
        e.array(chain.len() as u64)?;
        for cert in &chain {
            e.bytes(cert)?;
        }
        e.bytes(payload_bytes)?;
        e.bytes(&sig)?;
        Ok(w.position())
    })()
    .map_err(cbor_err)?;

    out.extend_from_slice(&cose_scratch[..cose_len])
        .map_err(|_| AttestError::BufferFull)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::time::Duration;
    use heapless::{String, Vec};
    use openprot_attest_api::consts::{
        MAX_CERT_SIZE, MAX_CHAIN_LEN, MAX_COMPONENT_LEN, MAX_DIGEST_LEN, MAX_MEASUREMENTS,
        MAX_TOKEN_SIZE, MAX_VERSION_LEN,
    };
    use openprot_attest_api::{AttestError, DigestAlgorithm, MeasurementAuthority, OemId};

    use crate::signer::STUB_CERT;

    struct TestSigner;

    impl HwSigner for TestSigner {
        fn sign(&self, _: &[u8]) -> Result<[u8; 96], AttestError> {
            Ok([0u8; 96])
        }
        fn cert_chain_der(
            &self,
            buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
        ) -> Result<(), AttestError> {
            let mut c0: Vec<u8, MAX_CERT_SIZE> = Vec::new();
            c0.extend_from_slice(&STUB_CERT).unwrap();
            let mut c1: Vec<u8, MAX_CERT_SIZE> = Vec::new();
            c1.extend_from_slice(&STUB_CERT).unwrap();
            buf.push(c0).map_err(|_| AttestError::BufferFull)?;
            buf.push(c1).map_err(|_| AttestError::BufferFull)?;
            Ok(())
        }
        fn caliptra_measurements(
            &self,
            _out: &mut Vec<openprot_attest_api::Measurement, MAX_MEASUREMENTS>,
        ) -> Result<(), AttestError> {
            Ok(())
        }
    }

    fn config() -> openprot_attest_api::AttestConfig {
        let mut hw_model: String<64> = String::new();
        hw_model.push_str("TestModel").unwrap();
        let mut oemid_bytes: Vec<u8, 16> = Vec::new();
        oemid_bytes
            .extend_from_slice(&[0x00, 0x01, 0x47, 0xae])
            .unwrap();
        openprot_attest_api::AttestConfig {
            oemid: OemId(oemid_bytes),
            hw_model,
            cert_cache_ttl: Duration::from_secs(3600),
        }
    }

    fn meas() -> Vec<Measurement, MAX_MEASUREMENTS> {
        let mut v = Vec::new();
        let mut component: String<MAX_COMPONENT_LEN> = String::new();
        component.push_str("Test ROM").unwrap();
        let mut version: String<MAX_VERSION_LEN> = String::new();
        version.push_str("1.0.0").unwrap();
        let mut digest: Vec<u8, MAX_DIGEST_LEN> = Vec::new();
        digest.extend_from_slice(&[0xAAu8; 48]).unwrap();
        v.push(Measurement {
            component,
            version,
            digest_alg: DigestAlgorithm::Sha384,
            digest,
            authority: MeasurementAuthority::Caliptra,
        })
        .unwrap();
        v
    }

    const STUB_UEID: [u8; crate::cert_ueid::UEID_LEN] = [0x01u8; crate::cert_ueid::UEID_LEN];

    fn build_token() -> Vec<u8, MAX_TOKEN_SIZE> {
        let mut out = Vec::new();
        build(
            &config(),
            &TestSigner,
            &STUB_UEID,
            &meas(),
            b"testnonce",
            &mut out,
        )
        .unwrap();
        out
    }

    fn decode_outer(token: &[u8]) -> (Vec<u8, 256>, Vec<u8, 256>) {
        // Minimal CBOR decode: 18([phdr-bstr, {33:[...]}, payload-bstr, sig-bstr])
        // Return (phdr_bytes, payload_bytes).
        let mut d = minicbor::Decoder::new(token);
        d.tag().unwrap(); // tag(18)
        d.array().unwrap(); // outer array len
        let phdr = d.bytes().unwrap();
        let mut phdr_v: Vec<u8, 256> = Vec::new();
        phdr_v.extend_from_slice(phdr).unwrap();
        d.skip().unwrap(); // unprotected header map (x5chain)
        let payload = d.bytes().unwrap();
        let mut payload_v: Vec<u8, 256> = Vec::new();
        payload_v.extend_from_slice(payload).unwrap();
        (phdr_v, payload_v)
    }

    fn cwt_decoder(payload: &[u8]) -> minicbor::Decoder<'_> {
        let mut d = minicbor::Decoder::new(payload);
        d.tag().unwrap(); // tag(55799) self-described CBOR
        d.tag().unwrap(); // tag(61) CWT
        d
    }

    fn find_claim_bytes(payload: &[u8], key: i64) -> Option<&[u8]> {
        let mut d = cwt_decoder(payload);
        let n = d.map().unwrap().unwrap_or(0);
        for _ in 0..n {
            let k = d.i64().unwrap();
            if k == key {
                return Some(d.bytes().unwrap());
            }
            d.skip().unwrap();
        }
        None
    }

    fn find_claim_str(payload: &[u8], key: i64) -> Option<&str> {
        let mut d = cwt_decoder(payload);
        let n = d.map().unwrap().unwrap_or(0);
        for _ in 0..n {
            let k = d.i64().unwrap();
            if k == key {
                return Some(d.str().unwrap());
            }
            d.skip().unwrap();
        }
        None
    }

    fn find_claim_i64(payload: &[u8], key: i64) -> Option<i64> {
        let mut d = cwt_decoder(payload);
        let n = d.map().unwrap().unwrap_or(0);
        for _ in 0..n {
            let k = d.i64().unwrap();
            if k == key {
                return Some(d.i64().unwrap());
            }
            d.skip().unwrap();
        }
        None
    }

    #[test]
    fn output_is_four_element_cbor_array() {
        let token = build_token();
        let mut d = minicbor::Decoder::new(&token);
        assert_eq!(d.tag().unwrap(), minicbor::data::Tag::new(18)); // COSE_Sign1
        assert_eq!(d.array().unwrap(), Some(4));
    }

    #[test]
    fn nonce_appears_in_payload() {
        let mut out = Vec::<u8, MAX_TOKEN_SIZE>::new();
        build(
            &config(),
            &TestSigner,
            &STUB_UEID,
            &meas(),
            b"testnonce",
            &mut out,
        )
        .unwrap();
        let (_, payload) = decode_outer(&out);
        assert_eq!(
            find_claim_bytes(&payload, CLAIM_NONCE),
            Some(b"testnonce" as &[u8])
        );
    }

    #[test]
    fn hw_model_in_payload() {
        let token = build_token();
        let (_, payload) = decode_outer(&token);
        assert_eq!(find_claim_str(&payload, CLAIM_HWMODEL), Some("TestModel"));
    }

    #[test]
    fn sign_receives_cose_sig_structure_for_payload() {
        // Verify that signer.sign() is called with the RFC 9052 §4.4
        // Sig_Structure: array(4) ["Signature1", phdr_bstr, h'', payload_bstr].
        use core::cell::RefCell;
        let captured: RefCell<heapless::Vec<u8, 2048>> = RefCell::new(heapless::Vec::new());

        struct CapturingSigner<'a>(&'a RefCell<heapless::Vec<u8, 2048>>);
        impl HwSigner for CapturingSigner<'_> {
            fn sign(&self, payload: &[u8]) -> Result<[u8; 96], AttestError> {
                let mut buf = self.0.borrow_mut();
                buf.clear();
                buf.extend_from_slice(&payload[..payload.len().min(2048)])
                    .unwrap();
                Ok([0u8; 96])
            }
            fn cert_chain_der(
                &self,
                buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
            ) -> Result<(), AttestError> {
                let mut c0: Vec<u8, MAX_CERT_SIZE> = Vec::new();
                c0.extend_from_slice(&STUB_CERT).unwrap();
                let mut c1: Vec<u8, MAX_CERT_SIZE> = Vec::new();
                c1.extend_from_slice(&STUB_CERT).unwrap();
                buf.push(c0).map_err(|_| AttestError::BufferFull)?;
                buf.push(c1).map_err(|_| AttestError::BufferFull)
            }
            fn caliptra_measurements(
                &self,
                _out: &mut Vec<openprot_attest_api::Measurement, MAX_MEASUREMENTS>,
            ) -> Result<(), AttestError> {
                Ok(())
            }
        }

        let signer = CapturingSigner(&captured);
        let mut out = Vec::<u8, MAX_TOKEN_SIZE>::new();
        build(
            &config(),
            &signer,
            &STUB_UEID,
            &meas(),
            b"testnonce",
            &mut out,
        )
        .unwrap();

        let cap = captured.borrow();
        let mut d = minicbor::Decoder::new(&cap);
        assert_eq!(d.array().unwrap(), Some(4)); // Sig_Structure is array(4)
        assert_eq!(d.str().unwrap(), "Signature1"); // context string
        assert!(!d.bytes().unwrap().is_empty()); // phdr_bstr (non-empty)
        assert!(d.bytes().unwrap().is_empty()); // aad = h''
                                                // payload_bstr decodes to tag(55799, tag(61, CWT map))
        let payload_bstr = d.bytes().unwrap();
        let mut pd = minicbor::Decoder::new(payload_bstr);
        assert_eq!(pd.tag().unwrap(), minicbor::data::Tag::new(55799));
        assert_eq!(pd.tag().unwrap(), minicbor::data::Tag::new(61));
    }

    #[test]
    fn nonce_outside_rfc9711_length_range_is_rejected() {
        let mut out = Vec::<u8, MAX_TOKEN_SIZE>::new();
        // Too short: 5 bytes < MIN_NONCE_LEN (8)
        let err = build(
            &config(),
            &TestSigner,
            &STUB_UEID,
            &meas(),
            b"short",
            &mut out,
        )
        .unwrap_err();
        assert!(matches!(err, AttestError::Caliptra(_)));
        // Too long: 65 bytes > MAX_NONCE_LEN (64)
        let err = build(
            &config(),
            &TestSigner,
            &STUB_UEID,
            &meas(),
            &[0xFFu8; 65],
            &mut out,
        )
        .unwrap_err();
        assert!(matches!(err, AttestError::Caliptra(_)));
    }

    #[test]
    fn dbgstat_ueid_oemid_and_sw_claims_match_config() {
        let token = build_token();
        let (_, payload) = decode_outer(&token);
        // dbgstat = 3 (disabled, hardcoded per OCP-EAT profile)
        assert_eq!(find_claim_i64(&payload, CLAIM_DBGSTAT), Some(3));
        // ueid matches STUB_UEID passed to build()
        assert_eq!(
            find_claim_bytes(&payload, CLAIM_UEID),
            Some(STUB_UEID.as_slice())
        );
        // oemid matches config
        assert_eq!(
            find_claim_bytes(&payload, CLAIM_OEMID),
            Some(&[0x00u8, 0x01, 0x47, 0xae][..])
        );
        // eat_profile OID matches spec
        assert_eq!(
            find_claim_bytes(&payload, CLAIM_EAT_PROFILE),
            Some(OCP_PROFILE_OID.as_slice())
        );
        // verify total fixed claims = 7
        let mut d = cwt_decoder(&payload);
        assert_eq!(d.map().unwrap(), Some(7));
    }
}
