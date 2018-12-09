//extern crate clap;
//use clap::{Arg, App};
//#[macro_use]
//extern crate itertools;
//#[macro_use]
//extern crate enum_primitive_derive;
#[macro_use]
extern crate bitflags;
extern crate num_traits;
extern crate getopts;
//extern crate mio;

mod stdio;
mod libc_utils;
mod poll;

use poll::{poll, PollFd, PollEvent, NOPOLLFD};
use getopts::Options;

//use mio::unix::{EventedFd, UnixReady};
//use mio::{Token, PollOpt, Ready, Poll, Events};
//use mio::net::TcpStream;

//use itertools::Itertools;

//use std::net::SocketAddr;

//use std::io::BufRead;
//use std::borrow::BorrowMut;
use std::{env, process};
use std::net::{TcpListener, TcpStream, UdpSocket, Shutdown};
use std::io;
//use std::io::ErrorKind;
//use std::os::unix::io::RawFd;

fn print_usage(program: &str, opts: Options, code: i32) {
    let brief = format!("Usage: {} [options] [destination] [port]", program);
    print!("{}", opts.usage(&brief));
    if code != 0 {
        process::exit(code);
    }
}

struct Opts<'a> {
    host: &'a str,
    port: &'a str,
    flags: Flags,
    family: Option<Family>,
    transport: Transport,
}

struct Flags {
    listen: bool,
    shutdown: bool,
}

enum Family {
    IpV4,
    IpV6,
    Unix,
}

enum Transport {
    Tcp,
    Udp,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help text");
    opts.optflag("l", "", "Listen mode, for inbound connects");
    opts.optflag("4", "", "Use IPv4");
    opts.optflag("6", "", "Use IPv6");
    opts.optflag("U", "", "Use UNIX domain socket");
    opts.optflag("u", "", "UDP mode");
    opts.optopt("I", "", "TCP receive buffer length", "length");
    opts.optopt("O", "", "TCP send buffer length", "length");
    opts.optflag("N", "", "Shutdown the network socket after EOF on stdin");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };
    if matches.opt_present("h") {
        print_usage(&program, opts, 0);
        return;
    }
    let flags = Flags {
        listen: matches.opt_present("l"),
        shutdown: matches.opt_present("N"),
    };

    let (opt_host, opt_port) = if matches.free.len() == 1 {
        if !flags.listen {
            print_usage(&program, opts, 1);
        }
        ("", matches.free[0].as_str())
    } else if matches.free.len() == 2 {
        (matches.free[0].as_str(), matches.free[1].as_str())
    } else {
        print_usage(&program, opts, 1);
        ("", "")
    };

    let opts = Opts {
        host: opt_host,
        port: opt_port,
        family: if matches.opt_present("4") {
            Some(Family::IpV4)
        } else if matches.opt_present("6") {
            Some(Family::IpV6)
        } else if matches.opt_present("U") {
            Some(Family::Unix)
        } else {
            None
        },
        transport: if matches.opt_present("u") {
            Transport::Udp
        } else {
            Transport::Tcp
        },
        flags: flags,
    };

    if let Err(err) = main_loop(&opts) {
        eprintln!("Error: {}", err);
        process::exit(1);
    };

    //let stream: Box<Write> = if flag_listen {
    //    listen(opt_host, opt_port);
    //    // TODO: Remove
    //    Box::new(TcpStream::connect(format!("{}:{}", opt_host, opt_port))?)
    //} else {
    //    //connect(opt_host, opt_port);
    //    Box::new(TcpStream::connect(format!("{}:{}", opt_host, opt_port)))
    //};
}

//fn tcp_connect(host: &str, port: &str) -> io::Result<TcpStream> {
//    let stream = NetTcpStream::connect(&format!("{}:{}", host, port))?;
//    TcpStream::from_stream(stream)
//}
//
//fn tcp_listen(host: &str, port: &str) -> io::Result<TcpStream> {
//    let stream = NetTcpStream::connect(&format!("{}:{}", host, port))?;
//    TcpStream::from_stream(stream)
//}

mod fd_io {
    use std::os::unix;
    use std::os::unix::io::RawFd;
    use std::io;
    //use std::cmp::min;
    use std::net::{TcpStream, UdpSocket, Shutdown};

    use stdio::{Stdin, Stdout};

    pub struct Buffer {
        buf: Vec<u8>,
        head: usize,
        tail: usize,
    }

    impl Buffer {
        pub fn new(len: usize) -> Self {
            Buffer {
                buf: vec![0; len],
                head: 0,
                tail: 0,
            }
        }

        pub fn empty(&self) -> bool {
            self.head == self.tail
        }

        //pub fn push(&mut self, buf: &[u8]) -> usize {
        //    let len = min(self.buf.len() - self.tail, buf.len());
        //    //let len = self.read(&mut buf.buf[buf.read..])?;
        //    self.buf[self.tail..self.tail + len].clone_from_slice(&buf[..len]);
        //    self.tail += len;
        //    len
        //}
    }

    pub trait Write {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize>;
        fn drain(&mut self, buf: &mut Buffer) -> io::Result<usize> {
            let len = self.write(&mut buf.buf[buf.head..buf.tail])?;
            buf.head += len;
            if buf.empty() {
                buf.head = 0;
                buf.tail = 0;
            }
            Ok(len)
        }
    }
    pub trait Read {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
        fn fill(&mut self, buf: &mut Buffer) -> io::Result<usize> {
            let len = self.read(&mut buf.buf[buf.tail..])?;
            buf.tail += len;
            Ok(len)
        }
    }
    pub trait AsRawFd {
        fn as_raw_fd(&self) -> RawFd;
    }
    pub trait Network {
        fn shutdown(&self, how: Shutdown) -> io::Result<()>;
    }

    impl Read for UdpSocket {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.recv(buf)
        }
    }
    impl Write for UdpSocket {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.send(buf)
        }
    }
    impl AsRawFd for UdpSocket {
        fn as_raw_fd(&self) -> RawFd {
            unix::io::AsRawFd::as_raw_fd(self)
        }
    }
    impl Network for UdpSocket {
        fn shutdown(&self, _how: Shutdown) -> io::Result<()> {
            Ok(())
        }
    }

    impl Read for TcpStream {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            io::Read::read(self, buf)
        }
    }
    impl Write for TcpStream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            io::Write::write(self, buf)
        }
    }
    impl AsRawFd for TcpStream {
        fn as_raw_fd(&self) -> RawFd {
            unix::io::AsRawFd::as_raw_fd(self)
        }
    }
    impl Network for TcpStream {
        fn shutdown(&self, how: Shutdown) -> io::Result<()> {
            TcpStream::shutdown(&self, how)
        }
    }

    impl Read for Stdin {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            io::Read::read(self, buf)
        }
    }
    impl AsRawFd for Stdin {
        fn as_raw_fd(&self) -> RawFd {
            unix::io::AsRawFd::as_raw_fd(self)
        }
    }

    impl Write for Stdout {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            io::Write::write(self, buf)
        }
    }
    impl AsRawFd for Stdout {
        fn as_raw_fd(&self) -> RawFd {
            unix::io::AsRawFd::as_raw_fd(self)
        }
    }

    pub trait Socket: Read + Write + AsRawFd + Network {}
    impl<T> Socket for T
    where
        T: Read + Write + AsRawFd + Network,
    {
    }

}

use fd_io::{Read, Write, AsRawFd, Socket, Buffer};

fn connect<'a>(
    host: &str,
    port: &str,
    family: &Option<Family>,
    transport: &Transport,
    listen: bool,
    //buf_netin: &Buffer,
) -> io::Result<Box<Socket>> {
    Ok(match transport {
        Transport::Tcp => {
            if listen {
                let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
                let (stream, _socket) = listener.accept()?;
                Box::new(stream)
            } else {
                let stream = TcpStream::connect(&format!("{}:{}", host, port))?;
                Box::new(stream)
            }
        }
        Transport::Udp => {
            if listen {
                let socket = UdpSocket::bind(format!("0.0.0.0:{}", port))?;
                let mut buf = vec![0; 2048];
                let (_, addr) = socket.peek_from(&mut buf)?;
                socket.connect(addr)?;
                Box::new(socket)
            } else {
                let socket = UdpSocket::bind(&"0.0.0.0:0")?;
                socket.connect(&format!("{}:{}", host, port))?;
                Box::new(socket)
            }
        }
    })
}

fn main_loop(opts: &Opts) -> io::Result<()> {
    //let mut stream = if opts.flags.listen {
    //    let listener = TcpListener::bind(format!("0.0.0.0:{}", opts.port))?;
    //    let (stream, _socket) = listener.accept()?;
    //    stream
    //} else {
    //    //tcp_connect(host, port)?;
    //    TcpStream::connect(&format!("{}:{}", opts.host, opts.port))?
    //};
    //let mut stream = TcpStream::from_stream(_stream)?;
    let mut stdin = stdio::Stdin::new()?;
    let mut stdout = stdio::Stdout::new()?;

    let mut buf_netin = Buffer::new(1024 * 64);
    let mut buf_stdin = Buffer::new(1024 * 64);

    let mut stream = connect(
        opts.host,
        opts.port,
        &opts.family,
        &opts.transport,
        opts.flags.listen,
        //&buf_netin,
    )?;
    //let mut buf_in = [0; 1024 * 64];
    //let (mut buf_in_read, mut buf_in_write) = (0, 0);
    //let mut buf_out = [0; 1024 * 64];
    //let (mut buf_out_read, mut buf_out_write) = (0, 0);

    //let mut stream_closed = false;
    //let mut stdin_closed = false;

    //let (POLL_STDIN, POLL_STDOUT, stream_idx) = (0, 1, 2);
    const POLL_STDIN: usize = 0;
    const POLL_NETOUT: usize = 1;
    const POLL_NETIN: usize = 2;
    const POLL_STDOUT: usize = 3;
    let mut pfd = {
        let poll_stdin = PollFd {
            fd: stdin.as_raw_fd(),
            events: PollEvent::POLLIN,
            revents: PollEvent::empty(),
        };
        let poll_netout = PollFd {
            fd: stream.as_raw_fd(),
            events: PollEvent::empty(),
            revents: PollEvent::empty(),
        };
        let poll_netin = PollFd {
            fd: stream.as_raw_fd(),
            events: PollEvent::POLLIN,
            revents: PollEvent::empty(),
        };
        let poll_stdout = PollFd {
            fd: stdout.as_raw_fd(),
            events: PollEvent::empty(),
            revents: PollEvent::empty(),
        };
        [poll_stdin, poll_netout, poll_netin, poll_stdout]
    };
    // NOPOLLFD = -1
    loop {
        pfd[POLL_STDIN].events.clear();
        pfd[POLL_NETOUT].events.clear();
        pfd[POLL_NETIN].events.clear();
        pfd[POLL_STDOUT].events.clear();
        if buf_netin.empty() {
            pfd[POLL_NETIN].events.insert(PollEvent::POLLIN);
        //eprintln!("NETIN POLLIN")
        } else {
            pfd[POLL_STDOUT].events.insert(PollEvent::POLLOUT);
            //eprintln!("STDOUT POLLOUT")
        }
        if buf_stdin.empty() {
            pfd[POLL_STDIN].events.insert(PollEvent::POLLIN);
        //eprintln!("STDIN POLLIN")
        } else {
            pfd[POLL_NETOUT].events.insert(PollEvent::POLLOUT);
            //eprintln!("NETOUT POLLOUT")
        }

        // Both inputs are gone, buffers are emtpy, we are done
        if pfd[POLL_NETIN].fd == NOPOLLFD && pfd[POLL_STDIN].fd == NOPOLLFD &&
            buf_netin.empty() && buf_stdin.empty()
        {
            break;
        }
        // Both outputs are gone, we can't continue
        if pfd[POLL_NETOUT].fd == NOPOLLFD && pfd[POLL_STDOUT].fd == NOPOLLFD {
            break;
        }
        // Listen and netin are gone, buffers are empty done, we are done
        if opts.flags.listen && pfd[POLL_NETIN].fd == NOPOLLFD && buf_netin.empty() &&
            buf_stdin.empty()
        {
            break;
        }

        poll(&mut pfd, None).unwrap();
        //eprintln!("POLL: {:?}", pfd);

        // Reading is possible after HUP, so we keep reading until we get POLLHUP and no POLLIN
        if pfd[POLL_STDIN].revents.contains(PollEvent::POLLHUP) &&
            !pfd[POLL_STDIN].revents.contains(PollEvent::POLLIN)
        {
            pfd[POLL_STDIN].fd = NOPOLLFD;
        }
        if pfd[POLL_NETIN].revents.contains(PollEvent::POLLHUP) &&
            !pfd[POLL_NETIN].revents.contains(PollEvent::POLLIN)
        {
            pfd[POLL_NETIN].fd = NOPOLLFD;
        }

        // Try to read from stdin
        if pfd[POLL_STDIN].revents.contains(PollEvent::POLLIN) {
            let len = stdin.fill(&mut buf_stdin)?;
            //eprintln!("stdin len = {}", len);
            if len == 0 {
                pfd[POLL_STDIN].fd = NOPOLLFD;
                //eprintln!("stdin EOF");
            }
        }
        // Try to write to network
        if pfd[POLL_NETOUT].revents.contains(PollEvent::POLLOUT) {
            //buf_stdin.drain::<Write>(stream.borrow_mut()).unwrap();
            stream.drain(&mut buf_stdin)?;
        }
        // Try to read from network
        if pfd[POLL_NETIN].revents.contains(PollEvent::POLLIN) {
            if stream.fill(&mut buf_netin)? == 0 {
                pfd[POLL_NETIN].fd = NOPOLLFD;
            }
        }
        // Try to write to stdout
        if pfd[POLL_STDOUT].revents.contains(PollEvent::POLLOUT) {
            stdout.drain(&mut buf_netin)?;
        }

        if pfd[POLL_NETOUT].revents.contains(PollEvent::POLLHUP) {
            if opts.flags.shutdown {
                stream.shutdown(Shutdown::Write).unwrap_or(());
            }
            pfd[POLL_NETOUT].fd = NOPOLLFD;
        }

        // Stdin gone and buf_netin empty
        if pfd[POLL_STDIN].fd == NOPOLLFD && buf_stdin.empty() {
            if opts.flags.shutdown {
                //eprintln!("stdin EOF -> shutdown");
                stream.shutdown(Shutdown::Write).unwrap_or(());
            }
            pfd[POLL_NETOUT].fd = NOPOLLFD;
        }
        // Netin gone and buf_netin empty
        if pfd[POLL_NETIN].fd == NOPOLLFD && buf_netin.empty() {
            pfd[POLL_STDOUT].fd = NOPOLLFD;
        }
    }
    return Ok(());
}

//fn listen(addr: &str, port: &str) {}
