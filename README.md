# Netcat in rust

This is a netcat implementation in rust, aiming for similar performance as original openbsd-netcat.

I originally started using mio, which in linux uses the epoll system call, but it had two problems:

1. Every loop requires several systemcalls beucase every file descriptor needs to be reregistered for every change in the polling events.
2. Regular files and others (like /dev/null) cannot be registered with epoll.

Problem 1. can be solved by using 4 epoll devices, one with each combination of file descriptor and polling event that is required.  I didn't find any solution to problem 2.

So I moved to using the poll system call, using a wrap over the libc function (note that openbsd-netcat also uses poll).

## Current status

Performance is similar to openbsd-netcat.

Here's the list of implemented options:

- [ ] -4		Use IPv4
- [ ] -6		Use IPv6
- [ ] -b		Allow broadcast
- [ ] -C		Send CRLF as line-ending
- [ ] -D		Enable the debug socket option
- [ ] -d		Detach from stdin
- [ ] -F		Pass socket fd
- [x] -h		This help text
- [ ] -I length	TCP receive buffer length
- [ ] -i interval	Delay interval for lines sent, ports scanned
- [ ] -k		Keep inbound sockets open for multiple connects
- [x] -l		Listen mode, for inbound connects
- [ ] -M ttl		Outgoing TTL / Hop Limit
- [ ] -m minttl	Minimum incoming TTL / Hop Limit
- [x] -N		Shutdown the network socket after EOF on stdin
- [ ] -n		Suppress name/port resolutions
- [ ] -O length	TCP send buffer length
- [ ] -P proxyuser	Username for proxy authentication
- [ ] -p port		Specify local port for remote connects
- [ ] -q secs		quit after EOF on stdin and delay of secs
- [ ] -r		Randomize remote ports
- [ ] -S		Enable the TCP MD5 signature option
- [ ] -s source	Local source address
- [ ] -T keyword	TOS value
- [ ] -t		Answer TELNET negotiation
- [ ] -U		Use UNIX domain socket
- [x] -u		UDP mode
- [ ] -V rtable	Specify alternate routing table
- [ ] -v		Verbose
- [ ] -W recvlimit	Terminate after receiving a number of packets
- [ ] -w timeout	Timeout for connects and final net reads
- [ ] -X proto	Proxy protocol: "4", "5" (SOCKS) or "connect"
- [ ] -x addr[:port]	Specify proxy address and port
- [ ] -Z		DCCP mode
- [ ] -z		Zero-I/O mode [used for scanning]

## Acknowledgements

- Nan Xiao for writing the gitbook [OpenBSD netcat demystified](https://nanxiao.gitbooks.io/openbsd-netcat-demystified/) which brought my attention to the design of netcat, and gave me the idea to reimplement it in rust.
- Israël Hallé and Daan Sprenkels for their libc's poll wrapper in rust, from which I've taken code for my own poll wrapper:
  - https://github.com/isra17/rust-poll
  - https://github.com/dsprenkels/rust-poll)
- Original nc implementer (*Hobbit* <hobbit@avian.org>) and OpenBSD developers (Eric Jackson <ericj@monkey.org> and Bob Beck) for the netcat code which I've used as a base (some parts are quite literal translations from C to Rust) for this project.
