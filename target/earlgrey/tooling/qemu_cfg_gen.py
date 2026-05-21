#!/usr/bin/env python3
# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

import argparse
import json
import sys
from configparser import ConfigParser
from typing import TextIO, Optional

# Helper to match cfggen's alphanumeric-ish sorting if needed.
# cfggen uses 'alphanum_key' from 'ot.util.misc'.
# Let's see if standard sorting is enough.
# The INI file had:
#   digest_const = ...
#   digest_iv = ...
#   flash_addr_const = ...
#   flash_addr_iv = ...
#   flash_data_const = ...
#   flash_data_iv = ...
#   inv_default_part_5 = ...
#   inv_default_part_6 = ...
#   inv_default_part_7 = ...
#   inv_default_part_8 = ...
#   inv_default_part_9 = ...
#   inv_default_part_10 = ...
#   scrmbl_key = ...
#   secret0_scramble_key = ...
# Standard sorting of these keys:
# ['digest_const', 'digest_iv', 'flash_addr_const', 'flash_addr_iv', 'flash_data_const', 'flash_data_iv', 'inv_default_part_10', 'inv_default_part_5', 'inv_default_part_6', 'inv_default_part_7', 'inv_default_part_8', 'inv_default_part_9', 'scrmbl_key', 'secret0_scramble_key']
# Wait! Standard sorting puts 'inv_default_part_10' BEFORE 'inv_default_part_5'!
# But in the generated INI, 'inv_default_part_10' was AFTER 'inv_default_part_9':
#   inv_default_part_5 = ...
#   inv_default_part_6 = ...
#   ...
#   inv_default_part_10 = ...
# This is because cfggen uses 'alphanum_key' which does natural sorting (numeric sorting for numbers).
# We should implement natural sorting to match exactly!


def try_int(s: str) -> getattr(str, "__class__", object):  # type: ignore
    try:
        return int(s)
    except ValueError:
        return s


def alphanum_key(s: str) -> list:
    import re

    return [try_int(c) for c in re.split(r"(\d+)", s)]


def add_pair(cfg: ConfigParser, devname: str, kname: str, value: str) -> None:
    if value:
        if f'ot_device "{devname}"' not in cfg:
            cfg[f'ot_device "{devname}"'] = {}
        cfg[f'ot_device "{devname}"'][f"  {kname}"] = f'"{value}"'


def generate_roms(
    cfg: ConfigParser, rom_ctrl: dict, socid: Optional[str] = None, count: int = 1
) -> None:
    for cnt in range(count):
        for rom, data in rom_ctrl.items():
            nameargs = ["ot-rom_ctrl"]
            if socid:
                if count > 1:
                    nameargs.append(f"{socid}{cnt}")
                else:
                    nameargs.append(socid)
            if rom != "null":
                nameargs.append(f"rom{rom}")
            romname = ".".join(nameargs)
            for kname in sorted(data.keys(), key=alphanum_key):
                val = data[kname]
                add_pair(cfg, romname, kname, val)


def generate_otp(
    cfg: ConfigParser, otp_ctrl: dict, variant: str, socid: Optional[str] = None
) -> None:
    nameargs = [f"ot-otp-{variant}"]
    if socid:
        nameargs.append(socid)
    otpname = ".".join(nameargs)
    for kname in sorted(otp_ctrl.keys(), key=alphanum_key):
        val = otp_ctrl[kname]
        add_pair(cfg, otpname, kname, val)


def generate_lc_ctrl(
    cfg: ConfigParser, lc_ctrl: dict, lc_states: dict, socid: Optional[str] = None
) -> None:
    nameargs = ["ot-lc_ctrl"]
    if socid:
        nameargs.append(socid)
    lcname = ".".join(nameargs)

    lcdata = {}
    # In cfggen, states are added first, then other constants, then sorted.
    # But ConfigParser preserves insertion order in Python 3.7+.
    # If we want to match exact sorting in output, we should collect all pairs first, then sort them.
    # Actually, cfggen did:
    #   for name, states in self._lcconst.states.items():
    #       self.add_pair(lcname, lcdata, f'{name}_first', states[0]) ...
    #   for kname, value in lc_ctrl.items():
    #       self.add_pair(lcname, lcdata, kname, value)
    #   lcdata = dict(sorted(lcdata.items()))
    # So the final keys are sorted.

    pairs = {}
    for name, states in lc_states.items():
        pairs[f"{name}_first"] = states[0]
        pairs[f"{name}_last"] = states[1]
    for kname, value in lc_ctrl.items():
        pairs[kname] = value

    for kname in sorted(pairs.keys(), key=alphanum_key):
        add_pair(cfg, lcname, kname, pairs[kname])


def generate_key_mgr(
    cfg: ConfigParser, keymgr: dict, socid: Optional[str] = None
) -> None:
    if not keymgr:
        return
    keymgr_name = keymgr.get("name", "keymgr")
    values = keymgr.get("values", {})
    nameargs = [f"ot-{keymgr_name}"]
    if socid:
        nameargs.append(socid)
    kmname = ".".join(nameargs)
    for kname in sorted(values.keys(), key=alphanum_key):
        value = values[kname]
        add_pair(cfg, kmname, kname, value)


def generate_ast(
    cfg: ConfigParser, top_clocks: dict, variant: str, socid: Optional[str] = None
) -> None:
    nameargs = [f"ot-ast-{variant}"]
    if socid:
        nameargs.append(socid)
    astname = ".".join(nameargs)

    topclockstr = ",".join(f'{name}:{c["frequency"]}' for name, c in top_clocks.items())
    aonclockstr = ",".join(name for name, c in top_clocks.items() if c["aon"])
    add_pair(cfg, astname, "topclocks", topclockstr)
    add_pair(cfg, astname, "aonclocks", aonclockstr)


def generate_clkmgr(
    cfg: ConfigParser,
    top_clocks: dict,
    sub_clocks: dict,
    clock_groups: dict,
    socid: Optional[str] = None,
) -> None:
    nameargs = ["ot-clkmgr"]
    if socid:
        nameargs.append(socid)
    clkname = ".".join(nameargs)

    refclocks = [name for name, c in top_clocks.items() if c["ref"]]
    if len(refclocks) > 1:
        raise ValueError(f'Multiple reference clocks detected: {", ".join(refclocks)}')

    clkrefname = refclocks[0] if refclocks else None
    clfrefval = top_clocks.get(clkrefname) if clkrefname else None

    topclockdefs = []
    # Sort clocks to match cfggen output (which seems to match input order, but let's sort to be deterministic if cfggen sorted them?
    # cfggen: `for ckname, ckval in self._top_clocks.items():`
    # In cfggen, self._top_clocks is populated in load_config, preserving insertion order from HJSON.
    # Our JSON also preserves insertion order (Python 3.7+ dict).
    # Let's see if sorting is needed. The INI had: "main:500,io:480,usb:240,aon:1".
    # In HJSON, clocks are defined in some order.
    # If we want to match exactly, we should preserve order.
    # In Python, dict preserves insertion order, so iterating over top_clocks.items() preserves order.
    for ckname, ckval in top_clocks.items():
        if clfrefval:
            clkratio = ckval["frequency"] // clfrefval["frequency"]
        else:
            clkratio = 1
        topclockdefs.append(f"{ckname}:{clkratio}")
    topclockstr = ",".join(topclockdefs)

    # subclocks in INI: "io_div2:io:2,io_div4:io:4,aes:main:1,hmac:main:1,kmac:main:1,otbn:main:1"
    # This also preserves insertion order from HJSON.
    subclockstr = ",".join(
        f'{name}:{c["source"]}:{c["div"]}' for name, c in sub_clocks.items()
    )

    # groups in INI: "powerup:aon+io+io_div2+io_div4+main+usb,trans:aes+hmac+kmac+otbn,..."
    # The group sources are sorted: `"+".join(sorted(g.sources))`
    # But the groups themselves preserve insertion order.
    groupstr = ",".join(
        f'{name}:{"+".join(sorted(g["sources"], key=alphanum_key))}'
        for name, g in clock_groups.items()
    )

    swcgstr = ",".join(name for name, g in clock_groups.items() if g["sw_cg"])
    hintstr = ",".join(name for name, g in clock_groups.items() if g["hint"])

    add_pair(cfg, clkname, "topclocks", topclockstr)
    if clkrefname:
        add_pair(cfg, clkname, "refclock", clkrefname)
    add_pair(cfg, clkname, "subclocks", subclockstr)
    add_pair(cfg, clkname, "groups", groupstr)
    add_pair(cfg, clkname, "swcg", swcgstr)
    add_pair(cfg, clkname, "hint", hintstr)


def generate_pwrmgr(
    cfg: ConfigParser, top_clocks: dict, socid: Optional[str] = None
) -> None:
    nameargs = ["ot-pwrmgr"]
    if socid:
        nameargs.append(socid)
    pwrname = ".".join(nameargs)

    clockstr = ",".join(name for name, c in top_clocks.items() if not c["aon"])
    add_pair(cfg, pwrname, "clocks", clockstr)


def main():
    parser = argparse.ArgumentParser(
        description="Regenerate QEMU config from JSON constants."
    )
    parser.add_argument("--json", required=True, help="Input JSON file with constants")
    parser.add_argument("--out", required=True, help="Output INI file")
    args = parser.parse_args()

    with open(args.json, "r") as f:
        data = json.load(f)

    cfg = ConfigParser()

    variant = data["variant"]

    rom_ctrl = data.get("rom_ctrl", {})
    generate_roms(cfg, rom_ctrl)

    otp_ctrl = data.get("otp_ctrl", {})
    generate_otp(cfg, otp_ctrl, variant)

    lc_ctrl = data.get("lc_ctrl", {})
    lc_states = data.get("lc_states", {})
    generate_lc_ctrl(cfg, lc_ctrl, lc_states)

    keymgr = data.get("keymgr", {})
    generate_key_mgr(cfg, keymgr)

    top_clocks = data.get("top_clocks", {})
    generate_ast(cfg, top_clocks, variant)

    sub_clocks = data.get("sub_clocks", {})
    clock_groups = data.get("clock_groups", {})
    generate_clkmgr(cfg, top_clocks, sub_clocks, clock_groups)

    generate_pwrmgr(cfg, top_clocks)

    with open(args.out, "w") as f:
        if data.get("git_version"):
            f.write(f'# Generated from OpenTitan commit: {data["git_version"]}\n\n')
        cfg.write(f)


if __name__ == "__main__":
    main()
