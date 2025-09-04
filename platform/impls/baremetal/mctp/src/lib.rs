#![no_std]
use mctp::{Eid, Error, MsgIC, MsgType, Result, Tag};
use mctp_estack::{Stack, fragment};

pub use mctp_estack::AppCookie;

pub const MAX_LISTENER_HANDLES: usize = 64;
pub const MAX_REQUEST_HANDLES: usize = 64;

#[derive(Debug)]
struct ReqHandle {
    /// Destination EID
    eid: Eid,
    /// Tag from last send
    ///
    /// Has to be cleared upon receiving a response.
    // A no-expire option might be added as a future improvement.
    last_tag: Option<Tag>,
}
impl ReqHandle {
    fn new(eid: Eid) -> ReqHandle {
        ReqHandle {
            eid,
            last_tag: None,
        }
    }
}

/// A platform agnostic MCTP stack with routing
#[derive(Debug)]
pub struct Router {
    stack: Stack,
    /// listener handles
    ///
    /// The index is used to construct the AppCookie.
    // TODO: A map with MsgType as key might be better.
    listeners: [Option<MsgType>; MAX_LISTENER_HANDLES],
    /// request handles
    ///
    /// The index is used to construct the AppCookie.
    requests: [Option<ReqHandle>; MAX_REQUEST_HANDLES],
}

impl Router {
    pub fn new<O>(own_eid: Eid, now_millis: u64, outbound: O) -> Self
    where
        O: FnMut(&[u8]),
    {
        // TODO: Outbound handler and lookup (a Trait might be a better fit)
        let stack = Stack::new(own_eid, now_millis);
        Router {
            stack,
            listeners: [None; MAX_LISTENER_HANDLES],
            requests: [const { None }; MAX_REQUEST_HANDLES],
        }
    }

    /// update the stack, returning after how many milliseconds update has to be called again
    pub fn update(&mut self, now_millis: u64) -> Result<u32> {
        // TODO: Handle timeouts
        self.stack.update(now_millis).map(|x| x.0 as u32)
    }

    /// Provide an incoming packet to the router.
    ///
    /// This expects a single MCTP packet, without transport binding header.
    pub fn inbound(&mut self, pkt: &[u8]) -> Result<()> {
        let own_eid = self.stack.eid();
        let Some(mut msg) = self.stack.receive(pkt)? else {
            return Ok(());
        };

        if msg.dest != own_eid {
            // Drop messages if eid does not match (for now)
            return Ok(());
        }

        match msg.tag {
            Tag::Unowned(_) => {
                // check for matching requests
                if let Some(cookie) = msg.cookie() {
                    if requests_index_from_cookie(cookie)
                        .is_some_and(|i| self.requests[i].is_some())
                    {
                        msg.retain();
                        return Ok(());
                    }
                }
                // In this case an unowned message that isn't associated to a request was received.
                // This might happen, if if this endpoint was inteded to route the packet to a different
                // bus it is connected to (bridge configuration).
                // Support for this is missing right now.
            }
            Tag::Owned(_) => {
                // check for matching listeners and retain with cookie
                for i in 0..self.listeners.len() {
                    if self.listeners[i] == Some(msg.typ) {
                        msg.set_cookie(Some(listener_cookie_from_index(i)));
                        msg.retain();
                        return Ok(());
                    }
                }
            }
        }

        // Return Ok(()) even if a message has been discarded
        Ok(())
    }

    /// Allocate a new request "_Handle_"
    pub fn req(&mut self, eid: Eid) -> Result<AppCookie> {
        for (index, handle) in self.requests.iter_mut().enumerate() {
            if handle.is_none() {
                let _ = handle.insert(ReqHandle::new(eid));
                return Ok(req_cookie_from_index(index));
            }
        }
        Err(mctp::Error::NoSpace)
    }

    /// Allocate a new listener for [`typ`](MsgType)
    pub fn listener(&mut self, typ: MsgType) -> Result<AppCookie> {
        if self.listeners.iter().any(|x| x == &Some(typ)) {
            return Err(mctp::Error::AddrInUse);
        }
        for (index, handle) in self.listeners.iter_mut().enumerate() {
            if handle.is_none() {
                let _ = handle.insert(typ);
                return Ok(listener_cookie_from_index(index));
            }
        }
        Err(mctp::Error::NoSpace)
    }

    /// Get the currently configured _Eid_ for this endpoint
    pub fn get_eid(&self) -> Eid {
        self.stack.eid()
    }

    /// Set the _Eid_ for this endpoint
    pub fn set_eid(&mut self, eid: Eid) -> Result<()> {
        self.stack.set_eid(eid.0)
    }

    pub fn send(
        &mut self,
        eid: Eid,
        typ: MsgType,
        tag: Option<Tag>,
        ic: MsgIC,
        cookie: AppCookie,
        bufs: &[&[u8]],
    ) -> Result<Tag> {
        const MTU: usize = 64;
        // TODO: mtu (and port) lookup
        let mut frag = self
            .stack
            .start_send(eid, typ, tag, true, ic, None, Some(cookie))?;

        let mut local_buffer = [0; mctp_estack::config::MAX_PAYLOAD];

        let payload = if bufs.len() == 1 {
            bufs[0]
        } else {
            let total_len = bufs.iter().fold(0, |acc, x| acc + x.len());
            if total_len > mctp_estack::config::MAX_PAYLOAD {
                return Err(Error::NoSpace);
            }
            let mut start = 0;
            for p in bufs {
                local_buffer[start..p.len()].copy_from_slice(p);
                start += p.len();
            }
            &local_buffer[..total_len]
        };
        // TODO: this seems unnecessary,
        // the fragmenter should iterate over the bufs requiring only a single packet buffer.

        loop {
            let mut pkt_buf = [0; MTU];
            match frag.fragment(payload, &mut pkt_buf) {
                fragment::SendOutput::Packet(items) => {
                    todo!("send data over the provided outgoing port")
                }
                fragment::SendOutput::Complete { tag, cookie: _ } => return Ok(tag),
                fragment::SendOutput::Error { err, cookie: _ } => return Err(err),
            }
        }
    }

    /// Receive a message associated with a [`AppCookie`]
    ///
    /// Returns `None` when no message is available for the listener/request.
    pub fn recv(&mut self, cookie: AppCookie) -> Option<mctp_estack::MctpMessage<'_>> {
        self.stack.get_deferred_bycookie(&[cookie])
    }

    /// Unbind a listener/request
    ///
    /// This has to be called to free the request/listener slot.
    /// Returns [BadArgument](Error::BadArgument) for cookies that are malformed or non existent.
    pub fn unbind(&mut self, cookie: AppCookie) -> Result<()> {
        if cookie_is_listener(&cookie) {
            self.listeners[listeners_index_from_cookie(cookie).ok_or(Error::BadArgument)?]
                .take()
                .ok_or(Error::BadArgument)?;
            Ok(())
        } else {
            let req = self.requests
                [requests_index_from_cookie(cookie).ok_or(Error::BadArgument)?]
            .take()
            .ok_or(Error::BadArgument)?;
            if let ReqHandle {
                eid,
                last_tag: Some(tag),
            } = req
            {
                self.stack.cancel_flow(eid, tag.tag());
            }
            Ok(())
        }
    }
}

/// Function to create a router unique AppCookie for listeners
///
/// Currently the listeners are just the index ranging from 0 to LISTENER_HANDLES-1.
/// Requests are enumerated from LISTENER_HANDLES to LISTENER_HANDLES+REQUEST_HANDLES-1
fn listener_cookie_from_index(i: usize) -> AppCookie {
    debug_assert!(
        i < MAX_LISTENER_HANDLES,
        "tried to create out of range listener AppCookie!"
    );
    AppCookie(i)
}

/// Function to create a router unique AppCookie for requests
///
/// Currently the listeners are just the index ranging from 0 to LISTENER_HANDLES-1.
/// Requests are enumerated from LISTENER_HANDLES to LISTENER_HANDLES+REQUEST_HANDLES-1
fn req_cookie_from_index(i: usize) -> AppCookie {
    debug_assert!(
        i < MAX_REQUEST_HANDLES,
        "tried to create out of range request AppCookie!"
    );
    AppCookie(i + MAX_LISTENER_HANDLES)
}

/// Get the listeners array index from a AppCookie
///
/// Returns `None` for invalid cookies.
fn listeners_index_from_cookie(cookie: AppCookie) -> Option<usize> {
    if cookie.0 < MAX_LISTENER_HANDLES {
        Some(cookie.0)
    } else {
        None
    }
}

/// Get the requesters array index from a AppCookie
///
/// Returns `None` for invalid cookies.
fn requests_index_from_cookie(cookie: AppCookie) -> Option<usize> {
    if cookie.0 >= MAX_LISTENER_HANDLES && cookie.0 < (MAX_LISTENER_HANDLES + MAX_REQUEST_HANDLES) {
        Some(cookie.0 - MAX_LISTENER_HANDLES)
    } else {
        None
    }
}

/// Check if a cookie is a corresponding to a listener
///
/// Checks based on the contained id.
/// Returns false for request cookies.
fn cookie_is_listener(cookie: &AppCookie) -> bool {
    cookie.0 < MAX_LISTENER_HANDLES
}

#[cfg(test)]
mod test {
    use mctp::Eid;

    use crate::Router;

    /// Test the creation of request and listener handles (`AppCookies`)
    #[test]
    fn test_handle_creation() {
        let mut router = Router::new(Eid(42), 0, |_| {});

        // create a new listener and expect the cookie value to be 0 (raw index of the underlying table)
        let listener = router.listener(mctp::MsgType(0));
        assert!(listener.is_ok());
        assert!(listener.as_ref().is_ok_and(|x| x.0 == 0));

        // create a new request
        // we expect the value to be MAX_LISTENER_HANDLES (request table index 0 + offset)
        let req = router.req(Eid(112));
        assert!(req.is_ok());
        assert!(
            req.as_ref()
                .is_ok_and(|x| x.0 == crate::MAX_LISTENER_HANDLES)
        );

        router
            .unbind(listener.unwrap())
            .expect("failed to unbind listener handle");
        router
            .unbind(req.unwrap())
            .expect("failed to unbind request handle");
    }
}
