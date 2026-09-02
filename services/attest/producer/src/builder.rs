// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! OCP-EAT COSE_Sign1 token assembly.
//!
//! Token structure (CBOR diagnostic notation):
//!
//! ```text
//! 18(                        ; COSE_Sign1
//!   [
//!     << { 1: -35, 33: [cert-chain...] } >>,   ; protected header
//!     {},                                        ; unprotected header
//!     << 61( { ...CWT claims... } ) >>,          ; payload (EAT CWT)
//!     h'...'                                     ; ES384 signature
//!   ]
//! )
//! ```
//!
//! Claim key numbers follow RFC 9711 and the OCP-EAT profile.

use heapless::Vec;
use minicbor::encode::Write as CborWrite;
use minicbor::Encoder;

use openprot_attest_api::consts::{MAX_CERT_SIZE, MAX_CHAIN_LEN, MAX_MEASUREMENTS, MAX_TOKEN_SIZE};
use openprot_attest_api::{AttestConfig, AttestError, DigestAlgorithm, HwSigner, Measurement};

// Registered EAT claim keys (RFC 9711 / RFC 8392)
const CLAIM_ISS: i64 = 1;
const CLAIM_IAT: i64 = 6;
const CLAIM_NONCE: i64 = 10;
const CLAIM_UEID: i64 = 256;
const CLAIM_OEMID: i64 = 258;
const CLAIM_HWMODEL: i64 = 259;
const CLAIM_HWVER: i64 = 260;
const CLAIM_DBGSTAT: i64 = 263;
const CLAIM_SWNAME: i64 = 14;
const CLAIM_SWVER: i64 = 15;
const CLAIM_MEASUREMENTS: i64 = -70000;
const CLAIM_EVIDENCE: i64 = -70001;

const ALG_ES384: i64 = -35;
const HDR_X5CHAIN: i64 = 33;

// Fixed scratch buffers used during token construction.
const SCRATCH: usize = MAX_TOKEN_SIZE;
// Sized to hold CBOR-encoded cert chain: each cert is bstr(MAX_CERT_SIZE) ≈ MAX_CERT_SIZE+3 bytes,
// plus the alg/x5chain map overhead.
const PHDR_SCRATCH: usize = MAX_CHAIN_LEN * (MAX_CERT_SIZE + 3) + 32;

/// Writer over a fixed `[u8]` slice; tracks how many bytes have been written.
struct BufWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> BufWriter<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }
}

impl<'a> CborWrite for BufWriter<'a> {
    type Error = AttestError;
    fn write_all(&mut self, data: &[u8]) -> Result<(), AttestError> {
        let end = self.pos + data.len();
        if end > self.buf.len() {
            return Err(AttestError::BufferFull);
        }
        self.buf[self.pos..end].copy_from_slice(data);
        self.pos = end;
        Ok(())
    }
}

/// Build and sign a complete OCP-EAT token into `out`.
///
/// `iat` is a Unix timestamp (seconds since epoch) supplied by the caller.
pub(crate) fn build(
    config: &AttestConfig,
    signer: &dyn HwSigner,
    measurements: &Vec<Measurement, MAX_MEASUREMENTS>,
    nonce: &[u8],
    evidence_cbor: &[u8],
    iat: u64,
    out: &mut Vec<u8, MAX_TOKEN_SIZE>,
) -> Result<(), AttestError> {
    // Fetch cert chain into a stack buffer.
    let mut chain: Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN> = Vec::new();
    signer.cert_chain_der(&mut chain)?;

    // ── Encode protected header ────────────────────────────────────────────
    // Must be encoded before signing so it can be included in Sig_Structure.
    let mut phdr_scratch = [0u8; PHDR_SCRATCH];
    let phdr_len = {
        let mut w = BufWriter::new(&mut phdr_scratch);
        let mut e = Encoder::new(&mut w);
        e.map(2).map_err(|_| AttestError::Cbor)?;
        e.i64(1).map_err(|_| AttestError::Cbor)?;
        e.i64(ALG_ES384).map_err(|_| AttestError::Cbor)?;
        e.i64(HDR_X5CHAIN).map_err(|_| AttestError::Cbor)?;
        e.array(chain.len() as u64).map_err(|_| AttestError::Cbor)?;
        for cert in &chain {
            e.bytes(cert).map_err(|_| AttestError::Cbor)?;
        }
        w.pos
    };
    let phdr_bytes = &phdr_scratch[..phdr_len];

    // ── Encode CWT claims map ──────────────────────────────────────────────
    let mut payload_scratch = [0u8; SCRATCH];
    let payload_len = {
        let mut w = BufWriter::new(&mut payload_scratch);
        let n_claims = 11 + usize::from(!evidence_cbor.is_empty());
        let mut e = Encoder::new(&mut w);
        e.tag(minicbor::data::Tag::new(61)).map_err(|_| AttestError::Cbor)?;
        e.map(n_claims as u64).map_err(|_| AttestError::Cbor)?;

        e.i64(CLAIM_ISS).map_err(|_| AttestError::Cbor)?;
        e.str("https://openprot.example/caliptra/device")
            .map_err(|_| AttestError::Cbor)?;

        e.i64(CLAIM_IAT).map_err(|_| AttestError::Cbor)?;
        e.i64(iat as i64).map_err(|_| AttestError::Cbor)?;

        e.i64(CLAIM_NONCE).map_err(|_| AttestError::Cbor)?;
        e.bytes(nonce).map_err(|_| AttestError::Cbor)?;

        // UEID: type RAND (0x01) + 28 placeholder bytes
        let mut ueid = [0u8; 29];
        ueid[0] = 0x01;
        e.i64(CLAIM_UEID).map_err(|_| AttestError::Cbor)?;
        e.bytes(&ueid).map_err(|_| AttestError::Cbor)?;

        e.i64(CLAIM_OEMID).map_err(|_| AttestError::Cbor)?;
        e.bytes(&config.oemid.0).map_err(|_| AttestError::Cbor)?;

        e.i64(CLAIM_HWMODEL).map_err(|_| AttestError::Cbor)?;
        e.str(&config.hw_model).map_err(|_| AttestError::Cbor)?;

        e.i64(CLAIM_HWVER).map_err(|_| AttestError::Cbor)?;
        e.array(2).map_err(|_| AttestError::Cbor)?;
        e.str(&config.hw_version).map_err(|_| AttestError::Cbor)?;
        e.i64(1).map_err(|_| AttestError::Cbor)?;

        // dbgstat = 3 (disabled)
        e.i64(CLAIM_DBGSTAT).map_err(|_| AttestError::Cbor)?;
        e.i64(3).map_err(|_| AttestError::Cbor)?;

        // sw-name array
        e.i64(CLAIM_SWNAME).map_err(|_| AttestError::Cbor)?;
        e.array(measurements.len() as u64)
            .map_err(|_| AttestError::Cbor)?;
        for m in measurements {
            e.str(&m.component).map_err(|_| AttestError::Cbor)?;
        }

        // sw-version array
        e.i64(CLAIM_SWVER).map_err(|_| AttestError::Cbor)?;
        e.array(measurements.len() as u64)
            .map_err(|_| AttestError::Cbor)?;
        for m in measurements {
            e.array(2).map_err(|_| AttestError::Cbor)?;
            e.str(&m.version).map_err(|_| AttestError::Cbor)?;
            e.i64(1).map_err(|_| AttestError::Cbor)?;
        }

        // measurements array
        e.i64(CLAIM_MEASUREMENTS).map_err(|_| AttestError::Cbor)?;
        e.array(measurements.len() as u64)
            .map_err(|_| AttestError::Cbor)?;
        for m in measurements {
            let alg: i64 = match m.digest_alg {
                DigestAlgorithm::Sha384 => -43,
                DigestAlgorithm::Sha512 => -44,
            };
            e.array(3).map_err(|_| AttestError::Cbor)?;
            e.str(&m.component).map_err(|_| AttestError::Cbor)?;
            e.i64(alg).map_err(|_| AttestError::Cbor)?;
            e.bytes(&m.digest).map_err(|_| AttestError::Cbor)?;
        }

        if !evidence_cbor.is_empty() {
            e.i64(CLAIM_EVIDENCE).map_err(|_| AttestError::Cbor)?;
            e.bytes(evidence_cbor).map_err(|_| AttestError::Cbor)?;
        }

        w.pos
    };
    let payload_bytes = &payload_scratch[..payload_len];

    // ── Sign ──────────────────────────────────────────────────────────────
    // RFC 9052 §4.4: signature input is Sig_Structure =
    //   ["Signature1", phdr_bstr, h'', payload_bstr]
    let sig = {
        let mut sig_scratch = [0u8; SCRATCH];
        let sig_input_len = {
            let mut w = BufWriter::new(&mut sig_scratch);
            let mut e = Encoder::new(&mut w);
            e.array(4).map_err(|_| AttestError::Cbor)?;
            e.str("Signature1").map_err(|_| AttestError::Cbor)?;
            e.bytes(phdr_bytes).map_err(|_| AttestError::Cbor)?;
            e.bytes(b"").map_err(|_| AttestError::Cbor)?; // aad = h''
            e.bytes(payload_bytes).map_err(|_| AttestError::Cbor)?;
            w.pos
        };
        signer.sign(&sig_scratch[..sig_input_len])?
    };

    // ── Assemble COSE_Sign1 ────────────────────────────────────────────────
    let mut cose_scratch = [0u8; SCRATCH];
    let cose_len = {
        let mut w = BufWriter::new(&mut cose_scratch);
        let mut e = Encoder::new(&mut w);
        e.tag(minicbor::data::Tag::new(18)).map_err(|_| AttestError::Cbor)?;
        e.array(4).map_err(|_| AttestError::Cbor)?;
        e.bytes(phdr_bytes).map_err(|_| AttestError::Cbor)?;
        e.map(0).map_err(|_| AttestError::Cbor)?; // empty unprotected header
        e.bytes(payload_bytes).map_err(|_| AttestError::Cbor)?;
        e.bytes(&sig).map_err(|_| AttestError::Cbor)?;
        w.pos
    };

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

    struct TestSigner;

    impl HwSigner for TestSigner {
        fn sign(&self, _: &[u8]) -> Result<[u8; 96], AttestError> {
            Ok([0u8; 96])
        }
        fn leaf_cert_der(&self, buf: &mut Vec<u8, MAX_CERT_SIZE>) -> Result<(), AttestError> {
            buf.extend_from_slice(&[0x30, 0x00])
                .map_err(|_| AttestError::BufferFull)
        }
        fn cert_chain_der(
            &self,
            buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
        ) -> Result<(), AttestError> {
            let mut c0: Vec<u8, MAX_CERT_SIZE> = Vec::new();
            c0.extend_from_slice(&[0x30, 0x00]).unwrap();
            let mut c1: Vec<u8, MAX_CERT_SIZE> = Vec::new();
            c1.extend_from_slice(&[0x30, 0x00]).unwrap();
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
        let mut hw_version: String<32> = String::new();
        hw_version.push_str("1.0.0").unwrap();
        let mut oemid_bytes: Vec<u8, 16> = Vec::new();
        oemid_bytes
            .extend_from_slice(&[0x00, 0x01, 0x47, 0xae])
            .unwrap();
        openprot_attest_api::AttestConfig {
            oemid: OemId(oemid_bytes),
            hw_model,
            hw_version,
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

    fn build_token(evidence: &[u8]) -> Vec<u8, MAX_TOKEN_SIZE> {
        let mut out = Vec::new();
        build(
            &config(),
            &TestSigner,
            &meas(),
            b"nonce",
            evidence,
            0,
            &mut out,
        )
        .unwrap();
        out
    }

    fn decode_outer(token: &[u8]) -> (Vec<u8, 256>, Vec<u8, 256>) {
        // Minimal CBOR decode: 18([phdr-bstr, {}, payload-bstr, sig-bstr])
        // Return (phdr_bytes, payload_bytes).
        let mut d = minicbor::Decoder::new(token);
        d.tag().unwrap(); // tag(18)
        d.array().unwrap(); // outer array len
        let phdr = d.bytes().unwrap();
        let mut phdr_v: Vec<u8, 256> = Vec::new();
        phdr_v.extend_from_slice(phdr).unwrap();
        d.skip().unwrap(); // empty map
        let payload = d.bytes().unwrap();
        let mut payload_v: Vec<u8, 256> = Vec::new();
        payload_v.extend_from_slice(payload).unwrap();
        (phdr_v, payload_v)
    }

    fn find_claim_bytes(payload: &[u8], key: i64) -> Option<&[u8]> {
        let mut d = minicbor::Decoder::new(payload);
        d.tag().unwrap(); // tag(61) CWT
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
        let mut d = minicbor::Decoder::new(payload);
        d.tag().unwrap(); // tag(61) CWT
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

    #[test]
    fn output_is_four_element_cbor_array() {
        let token = build_token(&[]);
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
            &meas(),
            b"testnonce",
            &[],
            0,
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
    fn empty_evidence_omits_evidence_claim() {
        let token = build_token(&[]);
        let (_, payload) = decode_outer(&token);
        assert!(find_claim_bytes(&payload, CLAIM_EVIDENCE).is_none());
    }

    #[test]
    fn non_empty_evidence_included_verbatim() {
        let evidence = [0xDE, 0xAD, 0xBE, 0xEF];
        let mut out = Vec::<u8, MAX_TOKEN_SIZE>::new();
        build(
            &config(),
            &TestSigner,
            &meas(),
            b"n",
            &evidence,
            0,
            &mut out,
        )
        .unwrap();
        let (_, payload) = decode_outer(&out);
        assert_eq!(
            find_claim_bytes(&payload, CLAIM_EVIDENCE),
            Some(&evidence[..])
        );
    }

    #[test]
    fn hw_model_in_payload() {
        let token = build_token(&[]);
        let (_, payload) = decode_outer(&token);
        assert_eq!(find_claim_str(&payload, CLAIM_HWMODEL), Some("TestModel"));
    }
}
