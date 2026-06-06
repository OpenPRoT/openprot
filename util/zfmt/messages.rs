// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use zfmt::Zfmt;

#[derive(Zfmt)]
#[zfmt(format = "ProcessStart: {name}")]
pub struct ProcessStart {
    pub name: &'static str,
}

#[derive(Zfmt)]
#[zfmt(format = "ProcessExit: {name} status={status:08x}")]
pub struct ProcessExit {
    pub name: &'static str,
    pub status: u32,
}
