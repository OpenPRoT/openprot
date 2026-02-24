# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

hsm = module_extension(
    implementation = lambda _: _hsm_repos(),
)

def _hsm_repos():
    http_archive(
        name = "cloud_kms_hsm",
        build_file = Label("//third_party/hsm:BUILD.cloud_kms_hsm.bazel"),
        url = "https://github.com/GoogleCloudPlatform/kms-integrations/releases/download/pkcs11-v1.8/libkmsp11-1.8-linux-amd64.tar.gz",
        strip_prefix = "libkmsp11-1.8-linux-amd64",
        sha256 = "3b932f22a8abb631442c3276e9c309554c81855526b74fbc9edaddcb57a557f7",
    )
