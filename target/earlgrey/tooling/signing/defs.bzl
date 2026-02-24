# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

load("//target/earlgrey/tooling/signing:keyset.bzl", _keyset = "keyset")
load(
    "//target/earlgrey/tooling/signing:signing.bzl",
    _sign_bin = "sign_bin",
    _sign_binary = "sign_binary",
)
load("//target/earlgrey/tooling/signing:tool.bzl", _signing_tool = "signing_tool")
load(
    "//target/earlgrey/tooling/signing:util.bzl",
    _KeySetInfo = "KeySetInfo",
    _SigningToolInfo = "SigningToolInfo",
)

# We re-export the two providers so other rules can use them.
SigningToolInfo = _SigningToolInfo
KeySetInfo = _KeySetInfo

# We re-export the following rules:
#   - keyset: Define keysets.
#   - signing_tool: describe a signing tool and configuration.
#   - sign_bin: sign a binary with a given key.
keyset = _keyset
signing_tool = _signing_tool
sign_bin = _sign_bin

# We also re-export the sign_binary function that can be used by other rules
# that may need to produce a signed binary (e.g. for deploying to a test environment).
sign_binary = _sign_binary
