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

load(
    "//target/earlgrey/tooling/signing:util.bzl",
    "KeySetInfo",
    "SigningToolInfo",
    _hsmtool_sign = "hsmtool_sign",
    _label_str = "label_str",
    _local_sign = "local_sign",
)

def _keyset(ctx):
    keys = {}
    config = {}
    for k, v in ctx.attr.keys.items():
        keyfile = k.files.to_list()

        # Parse the value (nickname and parameters) into a param dictionary
        # so we keep the per-key configuration parameters in the KeySetInfo
        # provider `config` field.
        param = v.split(":")
        if "=" not in param[0]:
            param[0] = "name=" + param[0]
        param = [p.split("=", 1) for p in param]
        param = {p[0]: p[1] for p in param}
        if len(keyfile) != 1:
            fail("keyset key labels must resolve to exactly one file.")
        keys[param["name"]] = keyfile[0]
        config[param["name"]] = param

    tool = ctx.attr.tool[SigningToolInfo]
    if tool.location == "local" and ctx.attr.profile != "local":
        print("WARNING: The selected signing tool {} cannot work with keyset profile `{}`.".format(
            _label_str(ctx.attr.tool.label),
            ctx.attr.profile,
        ))

    selected_key = ctx.build_setting_value
    if selected_key and selected_key not in keys:
        fail("Key name", selected_key, "is not in ", keys.keys())
    if ctx.attr.profile == "local":
        sign = _local_sign
    else:
        sign = _hsmtool_sign
    return [
        KeySetInfo(keys = keys, config = config, selected_key = selected_key, profile = ctx.attr.profile, sign = sign, tool = tool),
        DefaultInfo(files = depset(keys.values()), data_runfiles = ctx.runfiles(files = keys.values())),
    ]

keyset = rule(
    implementation = _keyset,
    build_setting = config.string(flag = True),
    attrs = {
        "keys": attr.label_keyed_string_dict(
            allow_files = True,
            mandatory = True,
            doc = "A mapping of key files to key names.  When a key file is a public key whose private component is held in an HSM, the name should be the same as the HSM label of that key.  Additional key parameters may be specified with colon-separated key=value pairs.",
        ),
        "profile": attr.string(
            mandatory = True,
            doc = "The hsmtool profile entry (in $XDG_CONFIG_HOME/hsmtool/profiles.json) associated with these keys or the value `local` for on-disk private keys.",
        ),
        "tool": attr.label(
            mandatory = True,
            providers = [SigningToolInfo],
        ),
    },
)
