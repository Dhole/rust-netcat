//extern crate clap;
//use clap::{Arg, App};
#[macro_use]
extern crate itertools;
#[macro_use]
extern crate enum_primitive_derive;
extern crate num_traits;
extern crate getopts;
extern crate mio;

mod stdio;

use getopts::Options;

use mio::unix::{EventedFd, UnixReady};

use mio::{Token, PollOpt, Ready, Poll, Events};
use mio::net::TcpStream;

use itertools::Itertools;

//use std::net::SocketAddr;

//use std::io::BufRead;
use std::env;
use std::process;
use std::net::TcpStream as NetTcpStream;
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

fn setup_stream(host: &str, port: &str) -> io::Result<TcpStream> {
    let stream = NetTcpStream::connect(&format!("{}:{}", host, port))?;
    TcpStream::from_stream(stream)
}

#[derive(Primitive, Clone, Copy)]
enum IoState {
    Read = 0,
    Write = 1,
}

fn main_loop(host: &str, port: &str, flag_listen: bool) -> io::Result<()> {
    //let stream: Box<Write> = if flag_listen {
    //    listen(opt_host, opt_port);
    //    // TODO: Remove
    //    Box::new(TcpStream::connect(format!("{}:{}", opt_host, opt_port))?)
    //} else {
    //    //connect(opt_host, opt_port);
    //    Box::new(TcpStream::connect(format!("{}:{}", opt_host, opt_port)))
    //};
    let mut stream = setup_stream(host, port)?;
    let stream_fd = stream.as_raw_fd();
    let stream_ev = EventedFd(&stream_fd);
    let _stdout = io::stdout();
    let mut stdout = _stdout.lock();
    let stdout_fd = _stdout.as_raw_fd();
    let stdout_ev = EventedFd(&stdout_fd);
    //let _stdin = io::stdin();
    //let mut stdin = _stdin.lock();
    let stdin = stdio::Stdin::new()?;
    let stdin_fd = stdin.as_raw_fd();
    let stdin_ev = EventedFd(&stdin_fd);
    let mut buf_in = [0; 1024 * 64];
    let mut buf_in_len = 0;
    let mut buf_out = [0; 1024 * 64];
    let mut buf_out_len = 0;

    //let mut stream_closed = false;
    let mut stdin_closed = false;

    const TOKEN_STDIN: Token = Token(0);
    const TOKEN_STDOUT: Token = Token(1);
    const TOKEN_STREAM: Token = Token(2);

    let poll_in_out = vec![
        vec![Poll::new()?, Poll::new()?],
        vec![Poll::new()?, Poll::new()?],
    ];
    for state_in in vec![IoState::Read, IoState::Write] {
        for state_out in vec![IoState::Read, IoState::Write] {
            let (in_ev, in_token, in_readyness) = match state_in {
                IoState::Read => (&stream_ev, TOKEN_STREAM, Ready::readable()),
                IoState::Write => (&stdout_ev, TOKEN_STDOUT, Ready::writable()),
            };
            let (out_ev, out_token, out_readyness) = match state_out {
                IoState::Read => (&stdin_ev, TOKEN_STDIN, Ready::readable()),
                IoState::Write => (&stream_ev, TOKEN_STREAM, Ready::writable()),
            };
            if in_token == out_token {
                poll_in_out[state_in as usize][state_out as usize]
                    .register(
                        in_ev,
                        in_token,
                        in_readyness | out_readyness | UnixReady::hup(),
                        PollOpt::level(),
                    )
                    .unwrap();
            } else {
                poll_in_out[state_in as usize][state_out as usize]
                    .register(
                        in_ev,
                        in_token,
                        in_readyness | UnixReady::hup(),
                        PollOpt::level(),
                    )
                    .unwrap();
                poll_in_out[state_in as usize][state_out as usize]
                    .register(
                        out_ev,
                        out_token,
                        out_readyness | UnixReady::hup(),
                        PollOpt::level(),
                    )
                    .unwrap();
            }
        }
    }
    let mut events = Events::with_capacity(1024);
    loop {
        let state_in = match buf_in_len {
            0 => IoState::Read,
            _ => IoState::Write,
        };
        let state_out = match buf_out_len {
            0 => IoState::Read,
            _ => IoState::Write,
        };
        poll_in_out[state_in as usize][state_out as usize]
            .poll(&mut events, None)
            .unwrap();
        for event in &events {
            //eprintln!("Event: {:?}", event);
            if event.readiness().is_readable() {
                match event.token() {
                    TOKEN_STDIN => {
                        //eprintln!("Read from stdin");
                        buf_out_len += stdin.read(&mut buf_out).unwrap();
                    }
                    TOKEN_STREAM => {
                        //eprintln!("Read from stream");
                        buf_in_len += stream.read(&mut buf_in).unwrap();
                        //eprintln!("Write_all to stdout");
                        //stdout.write_all(&buf_in[..len]).unwrap();
                        //stdout.flush().unwrap();
                    }
                    _ => unreachable!(),
                }
            }
            if event.readiness().is_writable() {
                match event.token() {
                    TOKEN_STREAM => {
                        //let len = stream.read(&mut buf_in).unwrap();
                        //eprintln!("Write to stream");
                        buf_out_len -= stream.write(&buf_out[..buf_out_len]).unwrap();
                    }
                    TOKEN_STDOUT => {
                        buf_in_len -= stdout.write(&buf_in[..buf_in_len]).unwrap();
                    }
                    _ => unreachable!(),
                }
            }
            if UnixReady::from(event.readiness()).is_hup() && event.token() == TOKEN_STREAM {
                //eprintln!("Stream closed");
                //stream_closed = true;
                return Ok(());
            }
            if UnixReady::from(event.readiness()).is_hup() && event.token() == TOKEN_STDIN {
                //eprintln!("Stdin closed");
                stdin_closed = true;
            }
        }
    }
}

//fn listen(addr: &str, port: &str) {}
