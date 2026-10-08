# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""The apps in the full update scenario.

Four of the five are the other scenarios' sources, built from here by
label: the agent, the firmware device and the bus from pldm_update, the
managed device from mock_bmc. Only the RoT is new, because it is the only
one that has to do both jobs at once.
"""

load("//target/ast10x0/tests/integration/mock_bmc:apps.bzl", mock_bmc_apps = "APPS")
load("//target/ast10x0/tests/integration/pldm_update:apps.bzl", _FLASH = "FLASH", pldm_apps = "APPS")

HERE = "//target/ast10x0/tests/integration/full_update:"

FLASH = _FLASH

ORCHESTRATOR_DEPS = [
    "//services/orchestrator/adapters/walk:orchestrator_checkpoint_walk",
    "//services/orchestrator/capabilities:orchestrator_capabilities",
    "//services/orchestrator/config:orchestrator_config",
    "//services/orchestrator/driver:orchestrator_driver",
    "//services/orchestrator/sm:orchestrator_sm",
    "//services/pldm/api:pldm_api",
    "//services/pldm/client:pldm_client",
    "//util/io",
    "//util/ipc:ipc",
    "@pigweed//pw_kernel/userspace",
    "@pigweed//pw_log/rust:pw_log",
    "@pigweed//pw_status/rust:pw_status",
]

def _without_orchestrator(apps):
    return [app for app in apps if app["name"] != "orchestrator"]

APPS = _without_orchestrator(pldm_apps) + _without_orchestrator(mock_bmc_apps) + [
    # The RoT: supervises the managed device and commands the firmware
    # device, which is what makes this scenario more than the other two
    # running side by side.
    {
        "deps": ORCHESTRATOR_DEPS,
        "name": "orchestrator",
        "src": HERE + "orchestrator_main.rs",
    },
]

TARGET_DEPS = ["//target/ast10x0/peripherals"]

SYSTEM_CONFIG = HERE + "system_config"
TARGET_SRC = HERE + "target.rs"
