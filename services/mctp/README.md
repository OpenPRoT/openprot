# MCTP Service

This directory contains the MCTP API, echo policy crate, and server implementation.

## Test Coverage

This section documents test target coverage (Bazel targets), not line or branch coverage percentages.

### Host Test Suite

Run all host-side tests with:

```bash
bazelisk test //services/mctp:mctp_host_tests --test_output=errors
```

`//services/mctp:mctp_host_tests` includes:

- `//services/mctp/api:mctp_api_test`
- `//services/mctp/echo:mctp_echo_host_test`
- `//services/mctp/server:mctp_server_dispatch_test`
- `//services/mctp/server:mctp_server_echo_test`
- `//services/mctp/server:mctp_server_integration_test`
- `//services/mctp/server:mctp_server_unit_test`

## Notes

- Transport-dependent behavior is covered in integration-style host tests using in-memory fixtures.
