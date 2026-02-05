# Copyright 2025 The Pigweed Authors
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not
# use this file except in compliance with the License. You may obtain a copy of
# the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
# WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
# License for the specific language governing permissions and limitations under
# the License.

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
