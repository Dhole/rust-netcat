extern crate libc;

// Code taken from std::sys
// I want access to unbuffered stdin, stdout and stderr.  I also would like to set them to non-blocking.
// The StdinRaw, StdoutRaw and StderrRaw from std::io are unbuffered but private.
// The Stdin, Stdout and Stderr from std::sys are public, but std::sys is private.
// My current solution is to copy the definitions of Stdin, Stdout and Stderr from std::sys in order to use them in my code.

use std::mem;
use std::cmp;
use std::io;
use stdio::libc::{c_void, c_int, ssize_t};
use std::os::unix::io::RawFd;
use std::os::unix::io::AsRawFd;
//use std::io::Read;
//use std::os::raw::c_int;
//use libc::c_int;

pub struct Stdin(FileDesc);
pub struct Stdout(());
pub struct Stderr(());

impl Stdin {
    pub fn new() -> io::Result<Stdin> {
        Ok(Stdin(FileDesc::new(libc::STDIN_FILENO)))
    }

    pub fn read(&self, data: &mut [u8]) -> io::Result<usize> {
        //let fd = FileDesc::new(libc::STDIN_FILENO);
        //let fd = self.0;
        //fd.into_raw();
        //ret
        self.0.read(data)
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.0.set_nonblocking(nonblocking)
    }
}

impl Stdout {
    pub fn new() -> io::Result<Stdout> {
        Ok(Stdout(()))
    }

    pub fn write(&self, data: &[u8]) -> io::Result<usize> {
        let fd = FileDesc::new(libc::STDOUT_FILENO);
        let ret = fd.write(data);
        fd.into_raw();
        ret
    }

    pub fn flush(&self) -> io::Result<()> {
        Ok(())
    }
}

impl Stderr {
    pub fn new() -> io::Result<Stderr> {
        Ok(Stderr(()))
    }

    pub fn write(&self, data: &[u8]) -> io::Result<usize> {
        let fd = FileDesc::new(libc::STDERR_FILENO);
        let ret = fd.write(data);
        fd.into_raw();
        ret
    }

    pub fn flush(&self) -> io::Result<()> {
        Ok(())
    }
}

impl AsRawFd for Stdin {
    fn as_raw_fd(&self) -> RawFd {
        libc::STDIN_FILENO
    }
}

impl AsRawFd for Stdout {
    fn as_raw_fd(&self) -> RawFd {
        libc::STDOUT_FILENO
    }
}

impl AsRawFd for Stderr {
    fn as_raw_fd(&self) -> RawFd {
        libc::STDERR_FILENO
    }
}

//impl Read for StdinRaw {
//    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
//        self.0.read(buf)
//    }
//
//    //#[inline]
//    //unsafe fn initializer(&self) -> Initializer {
//    //    Initializer::nop()
//    //}
//}

#[doc(hidden)]
trait IsMinusOne {
    fn is_minus_one(&self) -> bool;
}

macro_rules! impl_is_minus_one {
    ($($t:ident)*) => ($(impl IsMinusOne for $t {
        fn is_minus_one(&self) -> bool {
            *self == -1
        }
    })*)
}

impl_is_minus_one! { i8 i16 i32 i64 isize }

fn cvt<T: IsMinusOne>(t: T) -> io::Result<T> {
    if t.is_minus_one() {
        Err(io::Error::last_os_error())
    } else {
        Ok(t)
    }
}

#[derive(Debug)]
pub struct FileDesc {
    fd: c_int,
}

fn max_len() -> usize {
    // The maximum read limit on most posix-like systems is `SSIZE_MAX`,
    // with the man page quoting that if the count of bytes to read is
    // greater than `SSIZE_MAX` the result is "unspecified".
    //
    // On macOS, however, apparently the 64-bit libc is either buggy or
    // intentionally showing odd behavior by rejecting any read with a size
    // larger than or equal to INT_MAX. To handle both of these the read
    // size is capped on both platforms.
    if cfg!(target_os = "macos") {
        <c_int>::max_value() as usize - 1
    } else {
        <ssize_t>::max_value() as usize
    }
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
