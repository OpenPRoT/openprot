// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
//
// Regression fixtures for openprot-security.yml, run with:
//   semgrep --test .semgrep/rules
// Each `ruleid:` line must match its rule and each `ok:` line must not, so a
// rule that silently stops matching fails CI.

// ---- openprot-authenticated-svn-without-floor-check ----------------------

// ruleid: openprot-authenticated-svn-without-floor-check
fn admit_without_floor(v: Verdict) {
    match v {
        Verdict::Authenticated { svn } => store(svn),
        Verdict::Rejected => {}
    }
}

// ok: openprot-authenticated-svn-without-floor-check
fn admit_with_floor(v: Verdict, floor: &Floor) {
    let min = floor.floor();
    match v {
        Verdict::Authenticated { svn } if svn >= min => store(svn),
        _ => {}
    }
}

// ok: openprot-authenticated-svn-without-floor-check
fn no_verdict_here() {}

// ---- ast10x0-pkt-error-handling ------------------------------------------

impl Slave {
    fn handle_slave_interrupt(&mut self, status: u32) -> Option<Event> {
        // ruleid: ast10x0-pkt-error-handling
        if (status & constants::AST_I2CS_PKT_ERROR) != 0 {
            self.clear_slave_interrupts();
            return None;
        }
        // ok: ast10x0-pkt-error-handling
        if (status & constants::AST_I2CS_PKT_ERROR) != 0 {
            self.clear_slave_interrupts();
            self.arm_slave_receive(&mut cmd);
            return None;
        }
        None
    }

    fn unrelated(&mut self, status: u32) -> Option<Event> {
        // ok: ast10x0-pkt-error-handling
        if (status & constants::AST_I2CS_PKT_ERROR) != 0 {
            return None;
        }
        None
    }
}

// ---- ast10x0-irq-exact-match ---------------------------------------------

fn exact(sts: u32) {
    // ruleid: ast10x0-irq-exact-match
    if sts == constants::AST_I2CS_SLAVE_MATCH {}
    // ruleid: ast10x0-irq-exact-match
    if sts == A || sts == B {}
    // ok: ast10x0-irq-exact-match
    if (sts & constants::AST_I2CS_SLAVE_MATCH) != 0 {}
}

// ---- ast10x0-irq-todo-comment --------------------------------------------

fn todo() {
    // ruleid: ast10x0-irq-todo-comment
    // TODO packet slave sts
    // ok: ast10x0-irq-todo-comment
    // fully handled
}

// ---- openprot-decoded-length-unchecked -----------------------------------

fn decoded(buf: &[u8], hdr: &Hdr) {
    // ruleid: openprot-decoded-length-unchecked
    let _ = &buf[..hdr.payload_len as usize];
    // ruleid: openprot-decoded-length-unchecked
    let _ = &buf[8..8 + hdr.payload_len as usize];
    // ok: openprot-decoded-length-unchecked
    let _ = &buf[..Hdr::SIZE];
    // ok: openprot-decoded-length-unchecked
    let _ = &buf[..n];
}

// ---- openprot-type-cast-truncation ---------------------------------------

fn casts(status: u32, count: u32) {
    // ruleid: openprot-type-cast-truncation
    let _ = status as u8;
    // ok: openprot-type-cast-truncation
    let _ = count as u8;
}
