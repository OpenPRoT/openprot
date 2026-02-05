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

load("//target/earlgrey/tooling/signing:util.bzl", "SigningToolInfo")

def _signing_tool(ctx):
    env = {k: ctx.expand_location(v, ctx.attr.data) for k, v in ctx.attr.env.items()}
    return [SigningToolInfo(
        tool = ctx.executable.tool,
        data = ctx.files.data,
        env = env,
        location = ctx.attr.location,
    )]

signing_tool = rule(
    implementation = _signing_tool,
    attrs = {
        "data": attr.label_list(
            allow_files = True,
            cfg = "exec",
            doc = "Additional files needed by the signing tool",
        ),
        "env": attr.string_dict(
            doc = "Environment variables needed by the signing tool",
        ),
        "location": attr.string(
            mandatory = True,
            values = ["local", "token"],
            doc = "The location of private keys.  Local keys are on-disk and are typically used for simulation or emulation (FPGA) test scenarios.  Token keys are held in a secure token or HSM and are typically used for signing artifacts for real chips.",
        ),
        "tool": attr.label(
            mandatory = True,
            executable = True,
            allow_single_file = True,
            cfg = "exec",
            doc = "The signing tool binary",
        ),
    },
)
