// Code taken from std::sys
// I want access to unbuffered stdin, stdout and stderr.  I also would like to set them to non-blocking.
// The StdinRaw, StdoutRaw and StderrRaw from std::io are unbuffered but private.
// The Stdin, Stdout and Stderr from std::sys are public, but std::sys is private.
// My current solution is to copy the definitions of Stdin, Stdout and Stderr from std::sys in order to use them in my code.

#![allow(dead_code)]

extern crate libc;

use libc_utils::{cvt, max_len};

use std::mem;
use std::cmp;
use std::io;
use std::io::{Read, Write};
use self::libc::{c_void, c_int};
use std::os::unix::io::RawFd;
use std::os::unix::io::AsRawFd;
//use std::io::Read;
//use std::os::raw::c_int;
//use libc::c_int;

pub struct Stdin(FileDesc);
pub struct Stdout(FileDesc);
pub struct Stderr(FileDesc);

impl Stdin {
    pub fn new() -> io::Result<Stdin> {
        Ok(Stdin(FileDesc::new(libc::STDIN_FILENO)))
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.0.set_nonblocking(nonblocking)
    }
}

impl Read for Stdin {
    fn read(&mut self, data: &mut [u8]) -> io::Result<usize> {
        self.0.read(data)
    }

}

impl Stdout {
    pub fn new() -> io::Result<Stdout> {
        Ok(Stdout(FileDesc::new(libc::STDOUT_FILENO)))
    }
}

impl Write for Stdout {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.0.write(data)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Stderr {
    pub fn new() -> io::Result<Stderr> {
        Ok(Stderr(FileDesc::new(libc::STDERR_FILENO)))
    }
}

impl Write for Stderr {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.0.write(data)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl AsRawFd for Stdin {
    fn as_raw_fd(&self) -> RawFd {
        self.0.raw()
    }
}

impl AsRawFd for Stdout {
    fn as_raw_fd(&self) -> RawFd {
        self.0.raw()
    }
}

impl AsRawFd for Stderr {
    fn as_raw_fd(&self) -> RawFd {
        self.0.raw()
    }
}

#[derive(Debug)]
pub struct FileDesc {
    fd: c_int,
}

impl FileDesc {
    pub fn new(fd: c_int) -> FileDesc {
        FileDesc { fd: fd }
    }

    pub fn raw(&self) -> c_int {
        self.fd
    }

    /// Extracts the actual filedescriptor without closing it.
    pub fn into_raw(self) -> c_int {
        let fd = self.fd;
        mem::forget(self);
        fd
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let ret = cvt(unsafe {
            libc::read(
                self.fd,
                buf.as_mut_ptr() as *mut c_void,
                cmp::min(buf.len(), max_len()),
            )
        })?;
        Ok(ret as usize)
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let ret = cvt(unsafe {
            libc::write(
                self.fd,
                buf.as_ptr() as *const c_void,
                cmp::min(buf.len(), max_len()),
            )
        })?;
        Ok(ret as usize)
    }

    #[cfg(target_os = "linux")]
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        unsafe {
            let v = nonblocking as c_int;
            cvt(libc::ioctl(self.fd, libc::FIONBIO, &v))?;
            Ok(())
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        unsafe {
            let previous = cvt(libc::fcntl(self.fd, libc::F_GETFL))?;
            let new = if nonblocking {
                previous | libc::O_NONBLOCK
            } else {
                previous & !libc::O_NONBLOCK
            };
            if new != previous {
                cvt(libc::fcntl(self.fd, libc::F_SETFL, new))?;
            }
            Ok(())
        }
    }
}

impl Drop for FileDesc {
    fn drop(&mut self) {
        // Note that errors are ignored when closing a file descriptor. The
        // reason for this is that if an error occurs we don't actually know if
        // the file descriptor was closed or not, and if we retried (for
        // something like EINTR), we might close another valid file descriptor
        // opened after we closed ours.
        let _ = unsafe { libc::close(self.fd) };
    }
}
