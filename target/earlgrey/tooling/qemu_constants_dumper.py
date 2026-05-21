#!/usr/bin/env python3
# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""OpenTitan QEMU configuration dumper to JSON.

   Based on cfggen.py from lowRISC QEMU.
"""

from argparse import ArgumentParser
from configparser import ConfigParser
from logging import getLogger
from os.path import (
    abspath,
    basename,
    dirname,
    isdir,
    isfile,
    join as joinpath,
    normpath,
)
from traceback import format_exc
from typing import NamedTuple, Optional, TextIO
import sys
import json
import os

# Robust runfiles path resolution for Bazel
runfiles_dir = os.environ.get("RUNFILES_DIR")
if runfiles_dir:
    found = False
    for root, dirs, files in os.walk(runfiles_dir):
        if root.endswith(os.path.join("python", "qemu")) and "ot" in dirs:
            sys.path.append(root)
            found = True
            break
    if not found:
        print("Warning: Could not find qemu python path in runfiles", file=sys.stderr)
else:
    # Fallback
    QEMU_PYPATH = joinpath(
        dirname(dirname(dirname(normpath(__file__)))), "python", "qemu"
    )
    sys.path.append(QEMU_PYPATH)

try:
    _HJSON_ERROR = None
    from hjson import load as hjload
except ImportError as hjson_exc:
    _HJSON_ERROR = str(hjson_exc)

    def hjload(*_, **__):  # noqa: E301
        """dummy func if HJSON module is not available"""
        return {}


from ot.lc_ctrl.const import LcCtrlConstants
from ot.otp.const import OtpConstants
from ot.otp.secret import OtpSecretConstants
from ot.top import OpenTitanTop
from ot.util.arg import ArgError
from ot.util.log import configure_loggers
from ot.util.misc import alphanum_key, retrieve_git_version, to_bool


OtParamRegex = str
"""Definition of a parameter to seek and how to shorten it."""


class OtClock(NamedTuple):
    """Clock definition."""

    name: str
    """Clock signal name."""

    frequency: int
    """Clock frequency in Hz."""

    aon: bool
    """Whether the clock is always on."""

    ref: bool
    """Whether the clock is a reference clock."""


class OtDerivedClock(NamedTuple):
    """Clock derived from a top level clock definition."""

    name: str
    """Clock signal name."""

    source: str
    """Clock source signal name."""

    div: int
    """Divider."""


class OtClockGroup(NamedTuple):
    """Clock logicial group definition."""

    name: str
    """Group name."""

    sources: list[str]
    """Clock source signal names."""

    sw_cg: bool
    """Whether clock group can be managed by SW."""

    hint: bool
    """Whether clock group can be hinted by SW."""


class OtConfiguration:
    """QEMU configuration file generator."""

    MODULES = {
        "rom_ctrl": (True, r"RndCnstScr(.*)"),
        "otp_ctrl": (False, r"RndCnst(.*)Init"),
        "lc_ctrl": (False, r"RndCnstLcKeymgrDiv(.*)"),
        "keymgr": (
            False,
            r"RndCnst((?:.*)Seed)",
            # The CDI keymgr seed does not match 'RndCnst.*Seed'
            r"RndCnst(Cdi)",
        ),
        "keymgr_dpe": (False, r"RndCnst((?:.*)Seed)"),
    }

    TRANSLATIONS = {"keymgr": {"cdi": "cdi_seed"}}

    def __init__(self):
        self._log = getLogger("cfggen.cfg")
        self._otpconst = OtpConstants()
        self._lcconst = LcCtrlConstants()
        self._constants: dict[str, dict[Optional[int], dict[str, str]]] = {}
        self._top_clocks: dict[str, OtClock] = {}
        self._sub_clocks: dict[str, OtDerivedClock] = {}
        self._clock_groups: dict[str, OtClockGroup] = {}
        self._mod_clocks: dict[str, list[str]] = {}
        self._top_name: Optional[str] = None
        self._git_version: Optional[str] = None
        self._exclusions: dict[str, set[str]] = {}

    @property
    def top_name(self) -> Optional[str]:
        """Return the name of the top as defined in a configuration file."""
        return self._top_name

    def load_config(self, toppath: str) -> None:
        """Load data from HJSON configuration file."""
        assert not _HJSON_ERROR
        with open(toppath, "rt") as tfp:
            cfg = hjload(tfp, object_pairs_hook=dict)
        self._top_name = cfg.get("name")
        topbase = basename(toppath)

        self._git_version = retrieve_git_version(toppath)

        for module in cfg.get("module") or []:
            modtype = module.get("type")
            moddefs = self.MODULES.get(modtype)
            if not moddefs:
                continue
            multi, regexes = moddefs[0], moddefs[1:]
            consts = {}
            OtpSecretConstants.load_values(module, consts, multi, *regexes)
            if not consts:
                continue
            for cname, tname in self.TRANSLATIONS.get(modtype, {}).items():
                if cname in consts:
                    consts[tname] = consts.pop(cname)
            self._log.debug("Constants for %s loaded from %s", modtype, topbase)
            exist = modtype in self._constants
            if multi:
                if not exist:
                    self._constants[modtype] = consts
                else:
                    self._constants[modtype].update(consts)
            else:
                if exist:
                    raise ValueError(f"Redefinition of {modtype}")
                self._constants[modtype] = {None: consts}

        clocks = cfg.get("clocks", {})
        for clock in clocks.get("srcs", []):
            name = clock["name"]
            aon = to_bool(clock["aon"], False)
            ref = to_bool(clock["ref"], False)
            freq = int(clock["freq"])
            self._top_clocks[name] = OtClock(name, freq, aon, ref)
        for clock in clocks.get("derived_srcs", []):
            name = clock["name"]
            src = clock["src"]
            aon = to_bool(clock["aon"], False)
            freq = int(clock["freq"])
            div = int(clock["div"])
            src_clock = self._top_clocks.get(src)
            if not src_clock:
                raise ValueError(f"Invalid top clock {src} " f"referenced from {name}")
            if src_clock.frequency // div != freq:
                raise ValueError(
                    f"Incoherent derived clock {name} frequency: "
                    f"{src_clock.frequency}/{div} != {freq}"
                )
            if aon and not src_clock.aon:
                raise ValueError(f"Incoherent derived clock {name} AON")
            self._sub_clocks[name] = OtDerivedClock(name, src, div)
        clock_names = set(self._top_clocks.keys())
        clock_names.update(set(self._sub_clocks.keys()))
        for group in clocks.get("groups", []):
            ext = group["src"] == "ext"
            if ext:
                continue
            name = group["name"]
            hint = group["sw_cg"] == "hint"
            sw_cg = not hint and to_bool(group["sw_cg"], False)
            clk_srcs = []
            for clk_name, clk_src in group.get("clocks", {}).items():
                if not hint:
                    exp_name = f"clk_{clk_src}_{name}"
                    if clk_name != exp_name:
                        raise ValueError(
                            f"Unexpected clock {clk_name} in group"
                            f" {name} (exp: {exp_name})"
                        )
                    clk_srcs.append(clk_src)
                else:
                    exp_prefix = f"clk_{clk_src}_"
                    if not clk_name.startswith(exp_prefix):
                        raise ValueError(
                            f"Unexpected clock {clk_name} in group" f" {name}"
                        )
                    src_name = clk_name.removeprefix(exp_prefix)
                    clk_srcs.append(src_name)
                    if src_name in self._sub_clocks:
                        raise ValueError(f"Refinition of clock {src_name}")
                    self._sub_clocks[src_name] = OtDerivedClock(src_name, clk_src, 1)
            self._clock_groups[name] = OtClockGroup(name, clk_srcs, sw_cg, hint)
        modules = cfg.get("module", [])
        mod_clocks = {}
        for module in modules:
            type_ = module["type"]
            if type_ in ("ast", "clkmgr"):
                continue
            name = module["name"]
            clk_srcs = module.get("clock_srcs", {})
            clk_grp = module.get("clock_group", "")
            clocks = []
            for clk in clk_srcs.values():
                if isinstance(clk, dict):
                    clocks.append(f'{clk["group"]}.{clk["clock"]}')
                else:
                    clocks.append(f"{clk_grp}.{clk}")
            mod_clocks[name] = clocks
        self._mod_clocks = mod_clocks

    def load_lifecycle(self, svpath: str) -> None:
        """Load LifeCycle data from RTL file."""
        with open(svpath, "rt") as cfp:
            self._log.debug("Loading LC constants from %s", svpath)
            self._lcconst.load_sv(cfp)

    def load_otp_constants(self, svpath: str) -> None:
        """Load OTP data from RTL file."""
        with open(svpath, "rt") as cfp:
            self._log.debug("Loading OTP constants from %s", svpath)
            self._otpconst.load_sv(cfp)

    def load_constants(self, hjpath: Optional[str]) -> None:
        """Load definitions from HJSON file."""
        if not hjpath:
            return
        assert not _HJSON_ERROR
        self._log.debug("Loading secrets from %s", hjpath)
        hjbase = basename(hjpath)
        with open(hjpath, "rt") as tfp:
            cfg = hjload(tfp, object_pairs_hook=dict)
        for module in cfg.get("module") or []:
            modtype = module.get("type")
            moddefs = self.MODULES.get(modtype)
            if not moddefs:
                continue
            multi, regexes = moddefs[0], moddefs[1:]
            consts = {}
            OtpSecretConstants.load_values(module, consts, multi, *regexes)
            if not consts:
                continue
            for cname, tname in self.TRANSLATIONS.get(modtype, {}).items():
                if cname in consts:
                    consts[tname] = consts.pop(cname)
            self._log.debug("Constants for %s loaded from %s", modtype, hjbase)
            exist = modtype in self._constants
            if multi:
                if not exist:
                    self._constants[modtype] = consts
                else:
                    self._constants[modtype].update(consts)
            else:
                if exist:
                    raise ValueError(f"Redefinition of {modtype}")
                self._constants[modtype] = {None: consts}
        self._otpconst.load_secrets(cfg)

    def prepare(self) -> None:
        """Prepare generation of data, aggregating several sources."""
        digests = {
            "cnsty_digest": "digest",
            "flash_data_key": "flash_data",
            "flash_addr_key": "flash_addr",
            "sram_data_key": "sram",
        }
        avail_digests = self._otpconst.get_digests()
        otp_ctrl = self._constants["otp_ctrl"][None]
        for digest, prefix in digests.items():
            if digest not in avail_digests:
                continue
            pair = self._otpconst.get_digest_pair(digest, prefix)
            otp_ctrl.update(pair)
        for key in self._otpconst.get_scrambling_keys():
            key_value = self._otpconst.get_scrambling_key(key)
            key = key.removesuffix("key") + "_scramble_key"
            otp_ctrl[key] = key_value
        idx = 0
        while True:
            try:
                defaults = self._otpconst.get_partition_inv_defaults(idx)
                if defaults:
                    otp_ctrl[f"inv_default_part_{idx}"] = defaults
                idx += 1
            except ValueError:
                break
        lc_ctrl = self._constants["lc_ctrl"][None]
        lc_ctrl.update(self._lcconst.tokens)

    def exclude(self, exclusions: list[str]) -> None:
        """Add property exclusions."""
        for exclude in exclusions:
            try:
                dev, prop = exclude.split(".")
            except ValueError as exc:
                raise ArgError(f"Invalid exclusion format: {exclude}") from exc
            if dev not in self._exclusions:
                self._exclusions[dev] = set()
            self._exclusions[dev].add(prop)

    def dump_json(self, variant: str, ofp: TextIO) -> None:
        """Dump all configuration constants to JSON format."""
        data = {
            "variant": variant,
            "git_version": self._git_version,
            "rom_ctrl": self._constants.get("rom_ctrl", {}),
            "otp_ctrl": self._constants.get("otp_ctrl", {}).get(None, {}),
            "lc_ctrl": self._constants.get("lc_ctrl", {}).get(None, {}),
            "keymgr": {},
            "lc_states": {
                name: list(states) for name, states in self._lcconst.states.items()
            },
            "top_clocks": {
                c.name: {"frequency": c.frequency, "aon": c.aon, "ref": c.ref}
                for c in self._top_clocks.values()
            },
            "sub_clocks": {
                c.name: {"source": c.source, "div": c.div}
                for c in self._sub_clocks.values()
            },
            "clock_groups": {
                g.name: {"sources": g.sources, "sw_cg": g.sw_cg, "hint": g.hint}
                for g in self._clock_groups.values()
            },
        }

        for keymgr_name in ("keymgr", "keymgr_dpe"):
            if keymgr_name in self._constants:
                data["keymgr"] = {
                    "name": keymgr_name,
                    "values": self._constants[keymgr_name].get(None, {}),
                }
                break

        json.dump(data, ofp, indent=2)


def main():
    """Main routine"""
    debug = True
    try:
        argparser = ArgumentParser(description="Dump OpenTitan constants to JSON.")
        files = argparser.add_argument_group(title="Files")
        files.add_argument(
            "opentitan", nargs="?", metavar="OTDIR", help="OpenTitan root directory"
        )
        files.add_argument(
            "-T", "--top", choices=OpenTitanTop.names, help="OpenTitan top name"
        )
        files.add_argument(
            "-o",
            "--out",
            metavar="JSON",
            required=True,
            help="Filename of the JSON file to generate",
        )
        files.add_argument(
            "-c",
            "--otpconst",
            metavar="SV",
            help="OTP Constant SV file (default: auto)",
        )
        files.add_argument(
            "-l", "--lifecycle", metavar="SV", help="LifeCycle SV file (default: auto)"
        )
        files.add_argument(
            "-S", "--secrets", metavar="HJSON", help="Secret HJSON file (default: auto)"
        )
        files.add_argument(
            "-t",
            "--topcfg",
            metavar="HJSON",
            help="OpenTitan top HJSON config file " "(default: auto)",
        )
        mods = argparser.add_argument_group(title="Modifiers")
        mods.add_argument(
            "-x",
            "--exclude",
            action="append",
            metavar="DEVICE.NAME",
            default=[],
            help="Discard any property from DEVICE that starts "
            "with NAME (may be repeated)",
        )
        extra = argparser.add_argument_group(title="Extras")
        extra.add_argument("-v", "--verbose", action="count", help="increase verbosity")
        extra.add_argument(
            "-d", "--debug", action="store_true", help="enable debug mode"
        )
        args = argparser.parse_args()
        debug = args.debug

        log = configure_loggers(args.verbose, "cfggen", "lc", "otp")[0]

        if _HJSON_ERROR:
            argparser.error(f"Missing HJSON module: {_HJSON_ERROR}")

        cfg = OtConfiguration()

        topcfg = args.topcfg
        ot_dir = args.opentitan
        if not topcfg:
            if not args.opentitan:
                argparser.error("OTDIR is required is no top file is specified")
            if not isdir(ot_dir):
                argparser.error("Invalid OpenTitan root directory")
            ot_dir = abspath(ot_dir)
            if not args.top:
                argparser.error("Top name is required if no top file is " "specified")
            top = f"top_{args.top}"
            topvar = OpenTitanTop.short_name(args.top)
            topcfg = joinpath(ot_dir, f"hw/{top}/data/autogen/{top}.gen.hjson")
            if not isfile(topcfg):
                argparser.error(f"No such file '{topcfg}'")
            log.info("Top config: '%s'", topcfg)
            cfg.load_config(topcfg)
        else:
            if not isfile(topcfg):
                argparser.error(f"No such top file: {topcfg}")
            cfg.load_config(topcfg)
            ltop = cfg.top_name
            if not ltop:
                argparser.error("Unknown top name")
            log.info("Top: '%s'", ltop)
            ltop = ltop.lower()
            topvar = OpenTitanTop.short_name(cfg.top_name)
            if not topvar:
                argparser.error(f"Unsupported top name: {cfg.top_name}")
            top = f"top_{ltop}"
            if not ot_dir:
                check_dir = f"hw/{top}/data"
                cur_dir = dirname(topcfg)
                while cur_dir:
                    check_path = joinpath(cur_dir, check_dir)
                    if isdir(check_path):
                        ot_dir = cur_dir
                        break
                    cur_dir = dirname(cur_dir)
                if not ot_dir:
                    argparser.error("Cannot find OT root directory")
            elif not isdir(ot_dir):
                argparser.error("Invalid OpenTitan root directory")
            ot_dir = abspath(ot_dir)
            log.info("OT directory: '%s'", ot_dir)
        log.info("Variant: '%s'", topvar)
        top_dir = joinpath(ot_dir, "hw", top)

        lcfilename = "lc_ctrl_state_pkg.sv"
        lcpath = args.lifecycle
        if not lcpath:
            lc_constant_locations = [
                joinpath(top_dir, f"rtl/autogen/testing/{lcfilename}"),
                joinpath(top_dir, f"rtl/autogen/dev/{lcfilename}"),
                joinpath(top_dir, f"rtl/autogen/{lcfilename}"),
                joinpath(ot_dir, f"hw/ip/lc_ctrl/rtl/{lcfilename}"),
            ]
            for maybe_lcpath in lc_constant_locations:
                if isfile(maybe_lcpath):
                    lcpath = maybe_lcpath
                    break
        if not lcpath:
            argparser.error(f"Unknown location for '{lcfilename}'")
        if not isfile(lcpath):
            argparser.error(f"No such file '{lcpath}'")
        log.debug(f"'{lcfilename}' location: '%s'", lcpath)

        ocfilename = "otp_ctrl_part_pkg.sv"
        ocpath = args.otpconst
        if not ocpath:
            otp_constant_locations = [
                joinpath(top_dir, f"ip_autogen/otp_ctrl/rtl/{ocfilename}"),
                joinpath(ot_dir, f"hw/ip/otp_ctrl/rtl/{ocfilename}"),
            ]
            for maybe_ocpath in otp_constant_locations:
                if isfile(maybe_ocpath):
                    ocpath = maybe_ocpath
                    break
        if not ocpath:
            argparser.error(f"Unknown location for '{ocfilename}'")
        if not isfile(ocpath):
            argparser.error(f"No such file '{ocpath}'")
        log.debug(f"'{ocfilename}' location: '%s'", ocpath)

        secpath = args.secrets
        if secpath:
            if not isfile(secpath):
                argparser.error("No such secret file: {secpath}")
        else:
            sec_constant_locations = [
                joinpath(top_dir, f"data/autogen/{top}.secrets.testing.gen.hjson"),
                joinpath(top_dir, f"data/autogen/{top}.secrets.dev.gen.hjson"),
            ]
            for maybe_secpath in sec_constant_locations:
                if isfile(maybe_secpath):
                    secpath = maybe_secpath
                    break

        cfg.load_lifecycle(lcpath)
        cfg.load_otp_constants(ocpath)
        cfg.load_constants(secpath)
        cfg.prepare()
        cfg.exclude(args.exclude)

        with open(args.out, "wt") as ofp:
            cfg.dump_json(topvar, ofp)

    except (ArgError, IOError, ValueError, ImportError) as exc:
        print(f"\nError: {exc}", file=sys.stderr)
        if debug:
            print(format_exc(chain=False), file=sys.stderr)
        sys.exit(1)
    except KeyboardInterrupt:
        sys.exit(2)


if __name__ == "__main__":
    main()
