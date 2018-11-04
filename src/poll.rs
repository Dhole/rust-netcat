// Code mixed from https://github.com/isra17/rust-poll/blob/master/src/lib.rs
// and https://github.com/dsprenkels/rust-poll/blob/master/src/lib.rs


#![allow(dead_code)]

extern crate libc;

use libc_utils::cvt;

use std::io;
use std::os::unix::io::RawFd;

//use self::libc;

/// `nfds_t` as defined in poll.h
#[allow(non_camel_case_types)]
type nfds_t = libc::c_ulong;

bitflags! {
    #[repr(C)]
    pub struct PollEvent: u16 {
        const NONE    = 0x0000;
        /// There is data to read.
        const POLLIN  = 0x0001;
	/// There is urgent data to read.
        const POLLPRI = 0x0002;
	/// Writing now will not block.
        const POLLOUT = 0x0004;

	/// Error condition.
        const POLLERR = 0x0008;
	/// Hung up.
        const POLLHUP = 0x0010;
	/// Invalid polling request.
        const POLLNVAL= 0x0020;
    }
}

impl PollEvent {
    pub fn clear(&mut self) {
        self.bits = 0;
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PollFd {
    pub fd: RawFd, // file descriptor to poll
    pub events: PollEvent, // types of events poller cares about
    pub revents: PollEvent, // types of events that actually occurred
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PollResult {
    Some(i32),
    Timeout,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PollError {
    /// The array given as argument was not contained in the calling program's address space.
    EFAULT,
    /// A signal occurred before any requested event; see signal(7).
    EINTR,
    /// The nfds value exceeds the RLIMIT_NOFILE value.
    EINVAL,
    /// There was no space to allocate file descriptor tables.
    ENOMEM,
}

mod libc_poll {
    extern crate libc;

    use poll::nfds_t;
    use poll::PollFd;

    extern "C" {
        pub fn poll(__fds: *const PollFd, __nfds: nfds_t, __timeout: libc::c_int) -> libc::c_int;
    }
}

pub fn poll(fds: &mut [PollFd], timeout: Option<i32>) -> io::Result<PollResult> {
    let __timeout = match timeout {
        Some(t) => t,
        None => -1,
    } as libc::c_int;

    let ret = cvt(unsafe {
        libc_poll::poll(fds.as_ptr(), fds.len() as nfds_t, __timeout)
    })?;

    match ret {
        0 => Ok(PollResult::Timeout),
        n => Ok(PollResult::Some(n)),
    }
}
