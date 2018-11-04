//extern crate clap;
//use clap::{Arg, App};
//#[macro_use]
//extern crate itertools;
#[macro_use]
extern crate enum_primitive_derive;
#[macro_use]
extern crate bitflags;
extern crate num_traits;
extern crate getopts;
extern crate mio;

mod stdio;
mod libc_utils;
mod poll;

use poll::{poll, PollFd, PollEvent};
use getopts::Options;

use mio::unix::{EventedFd, UnixReady};
use mio::{Token, PollOpt, Ready, Poll, Events};
use mio::net::TcpStream;

//use itertools::Itertools;

//use std::net::SocketAddr;

//use std::io::BufRead;
use std::{env, process};
use std::net::{TcpListener, TcpStream as NetTcpStream};
use std::io::{self, Read, Write};
//use std::io::ErrorKind;
use std::os::unix::io::AsRawFd;

fn print_usage(program: &str, opts: Options, code: i32) {
    let brief = format!("Usage: {} [options] [destination] [port]", program);
    print!("{}", opts.usage(&brief));
    if code != 0 {
        process::exit(code);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help text");
    opts.optflag("l", "", "Listen mode, for inbound connects");
    opts.optflag("4", "", "Use IPv4");
    opts.optflag("6", "", "Use IPv6");
    opts.optopt("I", "", "TCP receive buffer length", "length");
    opts.optopt("O", "", "TCP send buffer length", "length");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };
    if matches.opt_present("h") {
        print_usage(&program, opts, 0);
        return;
    }
    let flag_listen = matches.opt_present("l");

    let (opt_host, opt_port) = if matches.free.len() == 1 {
        if !flag_listen {
            print_usage(&program, opts, 1);
        }
        ("", matches.free[0].as_str())
    } else if matches.free.len() == 2 {
        (matches.free[0].as_str(), matches.free[1].as_str())
    } else {
        print_usage(&program, opts, 1);
        ("", "")
    };

    if let Err(err) = main_loop(opt_host, opt_port, flag_listen) {
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

fn main_loop(host: &str, port: &str, flag_listen: bool) -> io::Result<()> {
    let _stream = if flag_listen {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
        let (stream, _socket) = listener.accept()?;
        stream
    } else {
        //tcp_connect(host, port)?;
        NetTcpStream::connect(&format!("{}:{}", host, port))?
    };
    let mut stream = TcpStream::from_stream(_stream)?;
    let stdin = stdio::Stdin::new()?;
    let stdout = stdio::Stdout::new()?;

    let mut buf_in = [0; 1024 * 64];
    let mut buf_in_len = 0;
    let mut buf_out = [0; 1024 * 64];
    let mut buf_out_len = 0;

    //let mut stream_closed = false;
    let mut stdin_closed = false;

    let (stdin_idx, stdout_idx, stream_idx) = (0, 1, 2);
    let mut fds = [
        PollFd {
            fd: stdin.as_raw_fd(),
            events: PollEvent::empty(),
            revents: PollEvent::empty(),
        },
        PollFd {
            fd: stdout.as_raw_fd(),
            events: PollEvent::empty(),
            revents: PollEvent::empty(),
        },
        PollFd {
            fd: stream.as_raw_fd(),
            events: PollEvent::empty(),
            revents: PollEvent::empty(),
        },
    ];
    loop {
        fds[stdin_idx].events = PollEvent::empty();
        fds[stdout_idx].events = PollEvent::empty();
        fds[stream_idx].events = PollEvent::empty();
        if buf_in_len == 0 {
            fds[stream_idx].events |= PollEvent::POLLIN;
        } else {
            fds[stdout_idx].events |= PollEvent::POLLOUT;
        }
        if buf_out_len == 0 {
            fds[stdin_idx].events |= PollEvent::POLLIN;
        } else {
            fds[stream_idx].events |= PollEvent::POLLOUT;
        }
        poll(&mut fds, None).unwrap();
        for fd_idx in 0..3 {
            //eprintln!("Event: {:?}", event);
            if fds[fd_idx].revents.contains(PollEvent::POLLIN) {
                if fd_idx == stdin_idx {
                    buf_out_len += stdin.read(&mut buf_out).unwrap();
                } else if fd_idx == stream_idx {
                    buf_in_len += stream.read(&mut buf_in).unwrap();
                }
            }
            if fds[fd_idx].revents.contains(PollEvent::POLLOUT) {
                if fd_idx == stream_idx {
                    buf_out_len -= stream.write(&buf_out[..buf_out_len]).unwrap();
                } else if fd_idx == stdout_idx {
                    buf_in_len -= stdout.write(&buf_in[..buf_in_len]).unwrap();
                }
            }
            if fds[fd_idx].revents.contains(PollEvent::POLLHUP) {
                if fd_idx == stream_idx {
                    return Ok(());
                } else if fd_idx == stdin_idx {
                    stdin_closed = true;
                }
            }
        }
    }
}

//fn listen(addr: &str, port: &str) {}
