# SPDM Evidence - AST1060-EVB Reference Implementation

This crate provides a **software-based reference implementation** of device measurements for SPDM attestation operations on the AST1060-EVB target.

> **Note:** This is a platform-specific implementation using fixed measurement values. Future hardware-backed implementations could integrate with TPM, boot measurements, or platform-specific attestation mechanisms.

## Overview

The evidence implementation provides device measurements used in the SPDM GET_MEASUREMENTS command. This allows SPDM requesters to verify the integrity and authenticity of the device.

## Architecture

```text
┌─────────────────────────┐
│  SPDM Requester         │
│  (Client)               │
└───────────┬─────────────┘
            │ GET_MEASUREMENTS
            ▼
┌─────────────────────────┐
│  SPDM Responder         │
└───────────┬─────────────┘
            │ pcr_quote()
            ▼
┌─────────────────────────┐
│  Ast1060Evidence        │◄── This crate
│  - Fixed measurements   │
│  - Format into PCR quote│
└─────────────────────────┘
```

## Features

- **Fixed measurements:** Two static string measurements for testing
- **No hardware dependencies:** Pure software implementation
- **Simple PCR quote format:** Easy to parse and verify
- **No_std compatible:** Embedded-friendly design

## Measurements

This implementation provides two fixed measurements:

1. **Measurement 0:** `"OpenPRoT SPDM Responder"` (23 bytes)
2. **Measurement 1:** `"OCP EMEA HELLO WORLD"` (20 bytes)

These are placeholder values for development and testing. Production implementations should provide real device measurements.

## Usage

### Basic Usage

```rust
use ast1060_evidence::Ast1060Evidence;
use spdm_lib::platform::evidence::SpdmEvidence;

// Create evidence provider
let evidence = Ast1060Evidence::new();

// Get required buffer size
let size = evidence.pcr_quote_size(false)?;

// Generate PCR quote
let mut buffer = vec![0u8; size];
let written = evidence.pcr_quote(&mut buffer, false)?;

// Buffer now contains formatted measurements
```

### Integration with SPDM Responder

```rust
use ast1060_evidence::Ast1060Evidence;
use spdm_lib::responder::SpdmResponder;

// Create evidence provider
let evidence = Ast1060Evidence::new();

// Create SPDM responder with evidence
let responder = SpdmResponder::new(
    transport,
    hash,
    rng,
    cert_store,
    evidence,  // ← Our implementation
);

// Handle GET_MEASUREMENTS request
// Responder will call evidence.pcr_quote() internally
```

## PCR Quote Format

The PCR quote is formatted as follows:

```
┌────────────────────────────────────────┐
│ Measurement Count (1 byte)             │  Value: 2
├────────────────────────────────────────┤
│ Measurement 0:                         │
│   - Index (1 byte)                     │  Value: 0
│   - Size (2 bytes, little-endian)      │  Value: 23
│   - Data (23 bytes)                    │  "OpenPRoT SPDM Responder"
├────────────────────────────────────────┤
│ Measurement 1:                         │
│   - Index (1 byte)                     │  Value: 1
│   - Size (2 bytes, little-endian)      │  Value: 20
│   - Data (20 bytes)                    │  "OCP EMEA HELLO WORLD"
└────────────────────────────────────────┘

Total size: 50 bytes
```

## Implementation Details

### Measurement Encoding

Each measurement is encoded as:
- **Index** (1 byte): Measurement index (0-based)
- **Size** (2 bytes): Length of measurement data (little-endian)
- **Data** (variable): Measurement value

### Memory Usage

- Fixed measurements: ~43 bytes (string data)
- Encoded output: 50 bytes
- No dynamic allocation required

### Post-Quantum Signatures

Post-quantum cryptography (PQC) signatures are not supported in this version. Calling `pcr_quote()` or `pcr_quote_size()` with `with_pqc_sig = true` will return `SpdmEvidenceError::UnsupportedEvidenceType`.

## Error Handling

| Error | Cause |
|-------|-------|
| `UnsupportedEvidenceType` | PQC signatures requested (not supported) |
| `InvalidEvidenceFormat` | Buffer too small for PCR quote |

## Testing

The implementation includes comprehensive unit tests:

```bash
# Build for AST1060-EVB platform
bazel build --platforms=//target/ast1060-evb:ast1060-evb \
    //target/ast1060-evb/evidence:evidence

# Run unit tests
bazel test --platforms=//target/ast1060-evb:ast1060-evb \
    //target/ast1060-evb/evidence:evidence
```

### Test Coverage

- ✅ Instance creation
- ✅ PCR quote size calculation
- ✅ PCR quote generation
- ✅ Measurement encoding
- ✅ Buffer validation
- ✅ PQC rejection
- ✅ Measurement value verification

## Future Hardware-Backed Implementations

When implementing hardware-backed versions:

### TPM Integration

```rust
impl Ast1060Evidence {
    fn pcr_quote(&self, buffer: &mut [u8], with_pqc_sig: bool) -> Result<usize> {
        // Read PCR values from TPM
        let pcr_values = platform::tpm::read_pcrs()?;

        // Generate TPM quote
        let quote = platform::tpm::quote(pcr_values)?;

        // Encode into SPDM format
        encode_tpm_quote(&quote, buffer)
    }
}
```

### Boot Measurement Integration

```rust
impl Ast1060Evidence {
    fn pcr_quote(&self, buffer: &mut [u8], with_pqc_sig: bool) -> Result<usize> {
        // Get boot measurements from measurement log
        let measurements = platform::boot::get_measurements()?;

        // Format for SPDM
        encode_measurements(&measurements, buffer)
    }
}
```

### Dynamic Measurements

```rust
// Support runtime measurement updates
impl Ast1060Evidence {
    pub fn extend_measurement(&mut self, index: u8, data: &[u8]) -> Result<()> {
        self.measurements[index as usize].extend(data)?;
        Ok(())
    }
}
```

## SPDM Protocol Usage

In SPDM protocol operations:

1. **GET_MEASUREMENTS Request**
   - Requester asks for device measurements
   - Optionally specifies measurement indices

2. **GET_MEASUREMENTS Response**
   - Responder calls `evidence.pcr_quote()`
   - Returns formatted measurements
   - May include signature over measurements

3. **Verification**
   - Requester validates measurements against expected values
   - Checks signature if present
   - Compares with trust anchor or policy

## Security Considerations

⚠️ **This implementation is for testing only:**

1. Fixed measurements provide no real attestation
2. No cryptographic binding to platform state
3. No signature over measurements
4. No hardware root of trust

### For Production:

1. Use real platform measurements (boot state, firmware, etc.)
2. Include cryptographic signatures (TPM quote signature)
3. Bind measurements to hardware identity
4. Implement measurement event log
5. Support measurement policies
6. Add anti-rollback protection

## Limitations

1. **Static measurements:** Values are fixed at compile time
2. **No signatures:** Measurements not cryptographically signed
3. **No PQC support:** Post-quantum signatures not implemented
4. **Simple format:** Not standard TPM quote format
5. **No measurement log:** No historical measurement tracking

## Example Output

When `pcr_quote()` is called, the buffer contains:

```
Offset  Value         Description
------  -----         -----------
0x00    0x02          Measurement count
0x01    0x00          Measurement 0 index
0x02    0x17 0x00     Size: 23 bytes (little-endian)
0x04    "OpenPRoT..." Measurement 0 data
0x1B    0x01          Measurement 1 index
0x1C    0x14 0x00     Size: 20 bytes (little-endian)
0x1E    "OCP EMEA..." Measurement 1 data
```

## Integration Example

Complete integration with SPDM responder:

```rust
use ast1060_evidence::Ast1060Evidence;
use ast1060_cert_store::Ast1060CertStore;
use openprot_spdm_hash::SpdmCryptoHash;
use openprot_spdm_rng::SpdmCryptoRng;

// Create all platform implementations
let cert_store = Ast1060CertStore::new(handle::CRYPTO);
let hash = SpdmCryptoHash::new(handle::CRYPTO);
let rng = SpdmCryptoRng::new(handle::CRYPTO);
let evidence = Ast1060Evidence::new();

// Create SPDM responder
let responder = SpdmResponder::new(
    transport,
    hash,
    rng,
    cert_store,
    evidence,
);

// Ready to handle SPDM requests
```

## License

Licensed under the Apache-2.0 license.
