// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! HACE command and algorithm constants shared across hash and digest modules.

pub const HACE_SHA_BE_EN: u32 = 1 << 3;
pub const HACE_CMD_ACC_MODE: u32 = 1 << 8;
pub const HACE_SG_EN: u32 = 1 << 18;
pub const HACE_SG_LAST: u32 = 1 << 31;
pub const HACE_ALGO_SHA256: u32 = (1 << 4) | (1 << 6);
pub const HACE_ALGO_SHA512: u32 = (1 << 5) | (1 << 6);
pub const HACE_ALGO_SHA384: u32 = (1 << 5) | (1 << 6) | (1 << 10);
pub const SHA256_HASH_CMD: u32 = HACE_CMD_ACC_MODE | HACE_SHA_BE_EN | HACE_SG_EN | HACE_ALGO_SHA256;
pub const SHA384_HASH_CMD: u32 = HACE_CMD_ACC_MODE | HACE_SHA_BE_EN | HACE_SG_EN | HACE_ALGO_SHA384;
pub const SHA512_HASH_CMD: u32 = HACE_CMD_ACC_MODE | HACE_SHA_BE_EN | HACE_SG_EN | HACE_ALGO_SHA512;

// ----- AES (crypto sub-engine) command bits -----------------------------
//
// Verbatim from the pinned authority `zephyr-reference/hace_aspeed.h` (the
// `HACE_CMD_*` macros; goal.md §1.9.2). `HACE_SG_LAST` (1<<31) above doubles
// as the crypto SG single/last terminator OR'd into `src/dst` SG length words
// (`hace_aspeed.c:132-133`).

/// `HACE_CMD_MBUS_REQ_SYNC_EN` (`hace_aspeed.h:17`).
pub const HACE_CMD_MBUS_REQ_SYNC_EN: u32 = 1 << 20;
/// `HACE_CMD_DES_SG_CTRL` (`hace_aspeed.h:18`).
pub const HACE_CMD_DES_SG_CTRL: u32 = 1 << 19;
/// `HACE_CMD_SRC_SG_CTRL` (`hace_aspeed.h:19`).
pub const HACE_CMD_SRC_SG_CTRL: u32 = 1 << 18;
/// `HACE_CMD_AES_KEY_HW_EXP` — hardware key expansion (`hace_aspeed.h:25`).
pub const HACE_CMD_AES_KEY_HW_EXP: u32 = 1 << 13;
/// `HACE_CMD_AES_SELECT == 0` (`hace_aspeed.h:22`).
pub const HACE_CMD_AES_SELECT: u32 = 0;
/// `HACE_CMD_ENCRYPT` (`hace_aspeed.h:28`); decrypt is the absence of this bit.
pub const HACE_CMD_ENCRYPT: u32 = 1 << 7;
/// `HACE_CMD_ECB == 0` (`hace_aspeed.h:29`).
pub const HACE_CMD_ECB: u32 = 0;
/// `HACE_CMD_CBC` (`hace_aspeed.h:30`).
pub const HACE_CMD_CBC: u32 = 0x1 << 4;
/// `HACE_CMD_AES128 == 0` (`hace_aspeed.h:34`).
pub const HACE_CMD_AES128: u32 = 0;
/// `HACE_CMD_AES256` (`hace_aspeed.h:36`).
pub const HACE_CMD_AES256: u32 = 0x2 << 2;

/// Fixed AES session base, `aspeed_crypto_session_setup` (`hace_aspeed.c:264`,
/// `:269`): SG control + MBUS sync + HW key expansion + AES select.
pub const AES_CMD_BASE: u32 = HACE_CMD_DES_SG_CTRL
    | HACE_CMD_SRC_SG_CTRL
    | HACE_CMD_MBUS_REQ_SYNC_EN
    | HACE_CMD_AES_KEY_HW_EXP
    | HACE_CMD_AES_SELECT;

pub const DEFAULT_POLL_BUDGET: u32 = 1_000_000;

/// Suggested wait window, in nanoseconds, passed to the cooperative `yield_fn`
/// between completion polls. Mirrors the reference HACE driver's 1 µs poll
/// interval (`reg_read_poll_timeout(..., 1, 3000)`). Advisory only: the
/// injected strategy decides whether/how to honor it (`spin_loop` ignores it;
/// an async/RTOS strategy may sleep for it).
pub const POLL_YIELD_NS: u32 = 1_000;
