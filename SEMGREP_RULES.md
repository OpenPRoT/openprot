# OpenPRoT Security Audit - Semgrep Rules Integration

## Overview
This document describes the semgrep rules created from the comprehensive OpenPRoT security audit findings. These rules are now integrated into the CI/CD pipeline to detect and prevent regressions of known vulnerabilities.

## Files Changed

### 1. `.semgrep/rules/openprot-security.yml`
New semgrep configuration file containing 6 rules organized by vulnerability category:

#### Anti-Rollback Enforcement
- **`openprot-authenticated-svn-without-floor-check`** (ERROR)
  - **Impact**: CRITICAL - Rollback bypass enabling downgrade to vulnerable firmware
  - **Scope**: `driver.rs`, `board.rs`
  - **Pattern**: Detects `Verdict::Authenticated` without floor validation
  - **Fix**: Always compare SVN against `SvnFloor::floor()` before accepting authenticated images

#### I2C Interrupt Handling
- **`ast10x0-pkt-error-handling`** (ERROR)
  - **Impact**: HIGH - Slave I2C receiver wedge on packet errors
  - **Scope**: `slave.rs`
  - **Pattern**: `PKT_ERROR` status path without `rearm_slave_rx()` or `arm_slave_receive()`
  - **Fix**: Re-arm receive DMA after clearing PKT_ERROR status

- **`ast10x0-irq-exact-match`** (WARNING)
  - **Impact**: HIGH - Silent frame drops without re-arm
  - **Scope**: `slave.rs`, `i2c.rs`
  - **Pattern**: Exact IRQ status matching (`sts == constant`) instead of bit tests
  - **Fix**: Use bitwise AND (`sts & constant`) to test individual bits with priority

- **`ast10x0-irq-todo-comment`** (ERROR)
  - **Impact**: HIGH - Unfinished interrupt handlers
  - **Scope**: `slave.rs`
  - **Pattern**: TODO comments in `handle_slave_interrupt()`
  - **Fix**: Complete all status paths with clear + re-arm

#### Buffer Boundary Validation
- **`openprot-decoded-length-unchecked`** (ERROR)
  - **Impact**: HIGH - Buffer overrun on oversized length fields
  - **Scope**: `wire.rs`, `protocol.rs`, `receiver.rs`, `dispatch.rs`
  - **Pattern**: Length fields from parsed headers used without bounds checking
  - **Fix**: Validate decoded length against buffer size before indexing

#### Memory Safety
- **`openprot-type-cast-truncation`** (WARNING)
  - **Impact**: MEDIUM - Silent bit loss from register operations
  - **Scope**: `slave.rs`, `i2c.rs`, `register*.rs`
  - **Pattern**: Type casts of bitmap/flags to narrower types (u8/u16)
  - **Fix**: Use exact-width types and explicit bit-preserving casts

### 2. `.github/workflows/ci.yml`
Updated security audit job to run both parsing-safety and OpenPRoT rules:

```yaml
- name: Run semgrep parsing-safety rules
  uses: returntocorp/semgrep-action@v1
  continue-on-error: true
  with:
    config: .semgrep/rules/parsing-safety.yml

- name: Run semgrep OpenPRoT security audit rules
  uses: returntocorp/semgrep-action@v1
  with:
    config: .semgrep/rules/openprot-security.yml
```

**Key Changes:**
- Separated parsing-safety and OpenPRoT rules into distinct steps
- OpenPRoT rules are blocking (will fail the build on findings)
- Parsing-safety rules continue on error (validation phase)

## Integration into CI

The semgrep rules are now run as part of the `security-audit` job in GitHub Actions:

1. **Trigger**: On every push to `main`/`develop` and pull requests to `main`/`earlgrey-hwe`
2. **Runner**: `ubuntu-latest`
3. **Execution Order**:
   - Generic Rust rules (p/rust) - informational
   - Parsing-safety rules - non-blocking validation
   - **OpenPRoT rules - blocking (will fail CI)**

## Findings Summary

The OpenPRoT security audit identified vulnerabilities in:

| Category | Finding | Severity | Status |
|----------|---------|----------|--------|
| Anti-Rollback | Unauthenticated SVN check | CRITICAL | Needs fix |
| I2C Interrupts | PKT_ERROR no re-arm | HIGH | Needs fix |
| I2C Interrupts | Exact-match IRQ status | HIGH | Needs fix |
| I2C Interrupts | TODO fallthrough | HIGH | Needs fix |
| Protocol Parsing | Oversized length fields | HIGH | Needs fix |
| Memory Safety | Type cast truncation | MEDIUM | Needs review |

## Next Steps

1. **Review Findings**: Run `semgrep scan --config .semgrep/rules/openprot-security.yml` to see current findings
2. **Fix Blocking Issues**: Address HIGH/CRITICAL rule violations before merging to main
3. **Continue-on-error**: Once initial fixes are in, consider setting `continue-on-error: false` for all rules
4. **Tune Rules**: Adjust patterns to reduce false positives based on codebase patterns

## How to Run Locally

```bash
# Install semgrep (if not already installed)
pip install semgrep

# Run all OpenPRoT rules
semgrep scan --config .semgrep/rules/openprot-security.yml

# Run specific rule
semgrep scan --config .semgrep/rules/openprot-security.yml -r openprot-authenticated-svn-without-floor-check

# Output as JSON for integration
semgrep scan --config .semgrep/rules/openprot-security.yml -f json
```

## Documentation References

- [OpenPRoT Security Report](./openprot_security_report_OpenPRoT.html)
- [Semgrep Rules Documentation](https://semgrep.dev/docs/rules/)
- [Custom Rule Writing Guide](https://semgrep.dev/docs/writing-rules/)
