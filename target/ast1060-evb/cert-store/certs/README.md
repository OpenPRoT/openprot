# Reference Certificates for AST1060-EVB

This directory contains reference X.509 certificates and keys for testing the SPDM certificate store implementation.

⚠️ **WARNING: FOR TESTING ONLY - DO NOT USE IN PRODUCTION** ⚠️

These certificates are provided as examples for development and testing. Production deployments must use properly provisioned certificates from a trusted Certificate Authority.

## Certificate Chain

### Root CA (Self-Signed)
- **File**: `root_ca_cert.pem` (PEM), `root_ca_cert.der` (DER)
- **Key**: `root_ca_key.pem` (PEM)
- **Subject**: CN=OpenPRoT Test Root CA, OU=Reference Implementation, O=OpenPRoT Project, L=Folsom, ST=California, C=US
- **Algorithm**: ECDSA P-384 with SHA-384
- **Validity**: 10 years
- **Key Usage**: Certificate Sign, CRL Sign
- **Basic Constraints**: CA:TRUE (critical)

### Leaf Certificate (Device)
- **File**: `leaf_cert.pem` (PEM), `leaf_cert.der` (DER)
- **Key**: `leaf_key.pem` (PEM), `leaf_key.der` (DER)
- **Raw Key**: `leaf_key_raw.bin` (48 bytes, SEC1 format)
- **Subject**: CN=AST1060-EVB Device, OU=Reference Implementation, O=OpenPRoT Project, L=Folsom, ST=California, C=US
- **Issuer**: OpenPRoT Test Root CA
- **Algorithm**: ECDSA P-384 with SHA-384
- **Validity**: 1 year
- **Key Usage**: Digital Signature (critical)
- **Extended Key Usage**: Server Auth, Client Auth

## Generated Files

### Certificate Files
- `root_ca_cert.pem` - Root CA certificate (PEM format)
- `root_ca_cert.der` - Root CA certificate (DER format, 715 bytes)
- `leaf_cert.pem` - Leaf certificate (PEM format)
- `leaf_cert.der` - Leaf certificate (DER format, 735 bytes)
- `cert_chain.pem` - Full chain (leaf + root, PEM)
- `cert_chain.der` - Full chain (leaf + root, DER, 1450 bytes)

### Key Files
- `root_ca_key.pem` - Root CA private key (PEM, ECDSA P-384)
- `leaf_key.pem` - Leaf private key (PEM, ECDSA P-384)
- `leaf_key.der` - Leaf private key (DER, SEC1 format, 167 bytes)
- `leaf_key_raw.bin` - Leaf private key (raw 48-byte scalar)

### Hash Files
- `root_hash.bin` - SHA-384 hash of root CA certificate (48 bytes)

### Other Files
- `leaf_csr.pem` - Certificate Signing Request (used during generation)
- `root_ca_cert.srl` - Serial number file (auto-generated)
- `openssl-root.cnf` - OpenSSL config for root CA
- `openssl-leaf.cnf` - OpenSSL config for leaf certificate

## File Sizes

| File | Size | Purpose |
|------|------|---------|
| `cert_chain.der` | 1450 bytes | Complete DER chain for SPDM |
| `root_hash.bin` | 48 bytes | SHA-384 root hash for SPDM |
| `leaf_key_raw.bin` | 48 bytes | Private key for signing |

## Using These Certificates

### Update cert-store Implementation

Edit `target/ast1060-evb/cert-store/src/lib.rs`:

```rust
// Replace placeholder data with real certificates
static SLOT_0_CERT_CHAIN: &[u8] = include_bytes!("../certs/cert_chain.der");
static SLOT_0_ROOT_HASH: [u8; SHA384_HASH_SIZE] = *include_bytes!("../certs/root_hash.bin");
static SLOT_0_PRIVATE_KEY: [u8; ECDSA_P384_PRIVATE_KEY_SIZE] = *include_bytes!("../certs/leaf_key_raw.bin");
const SLOT_0_CERT_CHAIN_LEN: usize = SLOT_0_CERT_CHAIN.len();
```

### Verify Certificate Chain

```bash
# Verify leaf cert against root CA
openssl verify -CAfile root_ca_cert.pem leaf_cert.pem

# Display certificate details
openssl x509 -in leaf_cert.pem -text -noout

# Display root CA details
openssl x509 -in root_ca_cert.pem -text -noout
```

### Extract Public Key from Leaf Certificate

```bash
# Extract public key for verification
openssl ec -in leaf_key.pem -pubout -out leaf_pubkey.pem

# Display public key
openssl ec -in leaf_pubkey.pem -pubin -text -noout
```

## SPDM Protocol Usage

In SPDM protocol operations:

1. **GET_DIGESTS**: Responder returns `root_hash.bin` (48 bytes)
2. **GET_CERTIFICATE**: Responder returns `cert_chain.der` (may be fragmented)
3. **CHALLENGE**: Responder signs challenge using `leaf_key_raw.bin`
4. **Verification**: Requester verifies signature using public key from `leaf_cert.der`

## Certificate Chain Format

The DER certificate chain (`cert_chain.der`) contains:

```
[Root CA Certificate (DER) - 715 bytes]
[Leaf Certificate (DER) - 735 bytes]
Total: 1450 bytes
```

When used in SPDM, this is wrapped with:
```
[SpdmCertChainHeader (4 bytes)]
[Root Hash (48 bytes)]
[Certificate Chain (1450 bytes)]
Total: 1502 bytes
```

## Security Considerations

⚠️ **These certificates are for testing only:**

1. Private keys are stored in plaintext files
2. Certificates are committed to source control
3. No hardware security module (HSM) protection
4. Keys have no access control
5. Not from a trusted Certificate Authority

### For Production:

1. Generate unique keys per device
2. Store private keys in OTP memory or HSM
3. Use keys from trusted CA or internal PKI
4. Implement key rotation
5. Protect against key extraction
6. Never commit private keys to version control

## Regenerating Certificates

To regenerate with different parameters:

```bash
# Generate new root CA
openssl ecparam -name secp384r1 -genkey -noout -out root_ca_key.pem
openssl req -new -x509 -sha384 -key root_ca_key.pem \
    -out root_ca_cert.pem -days 3650 -config openssl-root.cnf

# Generate new leaf certificate
openssl ecparam -name secp384r1 -genkey -noout -out leaf_key.pem
openssl req -new -sha384 -key leaf_key.pem \
    -out leaf_csr.pem -config openssl-leaf.cnf
openssl x509 -req -sha384 -in leaf_csr.pem \
    -CA root_ca_cert.pem -CAkey root_ca_key.pem -CAcreateserial \
    -out leaf_cert.pem -days 365 -extfile openssl-leaf.cnf -extensions v3_req

# Convert to DER
openssl x509 -in root_ca_cert.pem -outform DER -out root_ca_cert.der
openssl x509 -in leaf_cert.pem -outform DER -out leaf_cert.der
cat root_ca_cert.der leaf_cert.der > cert_chain.der

# Generate root hash
openssl dgst -sha384 -binary root_ca_cert.der > root_hash.bin

# Extract raw private key (48 bytes at offset 7 in DER)
openssl ec -in leaf_key.pem -outform DER -out leaf_key.der
dd if=leaf_key.der of=leaf_key_raw.bin bs=1 skip=7 count=48
```

## License

These reference certificates are provided under the Apache-2.0 license for testing purposes only.
