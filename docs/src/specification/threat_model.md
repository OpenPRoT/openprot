# Threat Model

## Assets

-   Integrity and authenticity of OpenPRoT firmware
-   Integrity and authorization of cryptographic operations
-   Integrity of anti-rollback counters
-   Integrity and confidentiality of symmetric keys managed by OpenPRoT
-   Integrity and confidentiality of private asymmetric keys
-   Integrity of boot measurements
-   Integrity and authenticity of firmware update payloads
-   Integrity and authenticity of OpenPRoT policies

## Attacker Profile

The attack profile definition is based on the JIL Application of Attack
Potential to Smartcards and Similar Devices Specification version 3.2.1.

-   **Type of access**: physical, remote
-   **Attacker Proficiency Levels**: expert, proficient, laymen
-   **Knowledge of the TOE**: public (open source), critical for signing keys
-   **Equipment**: none, standard, specialized, bespoke

### Attacks within Scope

See the JIL specification for examples of attacks.

-   Physical attacks
-   Perturbation attacks
-   Side-channel attacks
-   Exploitation of test features
-   Attacks on RNG
-   Software attacks
-   Application isolation

## Threat Modeling

To provide a transparent view of the security posture for a given OpenPRoT +
hardware implementation, integrators are required to perform a threat modeling
analysis. This analysis must evaluate the specific implementation against the
assets and attacker profile defined in this document.

The results of this analysis must be documented in table format, with the
following columns:

-   **Threat ID**: Unique identifier which can be referenced in documentation and
    security audits
-   **Threat Description**: Definition of the attack profile and potential attack.
-   **Target Assets**: List of impacted assets
-   **Mitigation(s)**: List of countermeasures implemented in hardware and/or
    software to mitigate the potential attack
-   **Verification**: Results of verification plan used to gain confidence in the
    mitigation strategy.

Integrators should use the JIL specification as a guideline to identify relevant
attacks and must detail the specific mitigation strategies implemented in their
design. The table must be populated for the target hardware implementation to
allow for a comprehensive security review.
