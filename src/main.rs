//extern crate clap;
//use clap::{Arg, App};
extern crate getopts;
extern crate mio;

use getopts::Options;

use mio::unix::EventedFd;

use mio::{Token, PollOpt, Ready, Poll, Events};

use std::env;
use std::process;
use std::net::TcpStream;
use std::io::{self, Read, Write};
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
    TcpStream::connect(format!("{}:{}", host, port))
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
    let _stdout = io::stdout();
    let mut stdout = _stdout.lock();
    let _stdin = io::stdin();
    let mut stdin = _stdin.lock();
    let mut buf_in = [0; 1024];

    const TOKEN_STDIN: Token = Token(0);
    const TOKEN_STREAM: Token = Token(1);
    let poll = Poll::new()?;
    poll.register(
        &EventedFd(&_stdin.as_raw_fd()),
        TOKEN_STDIN,
        Ready::readable(),
        PollOpt::edge(),
    )?;
    poll.register(
        &EventedFd(&stream.as_raw_fd()),
        TOKEN_STREAM,
        Ready::readable(),
        PollOpt::edge(),
    )?;
    let mut events = Events::with_capacity(1024);
    loop {
        poll.poll(&mut events, None)?;
        for event in &events {
            match event.token() {
                TOKEN_STDIN => {
                    let len = stdin.read(&mut buf_in)?;
                    stream.write(&buf_in[..len])?;
                    stream.flush()?;
                }
                TOKEN_STREAM => {
                    let len = stream.read(&mut buf_in)?;
                    stdout.write(&buf_in[..len])?;
                    stdout.flush()?;
                }
                _ => unreachable!(),
            }
        }
    }
}

//fn listen(addr: &str, port: &str) {}
