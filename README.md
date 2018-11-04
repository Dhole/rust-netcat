# Netcat in rust

This is a netcat implementation in rust, aiming for similar performance as original openbsd-netcat.

I originally started using mio, which in linux uses the epoll system call, but it had two problems:

1. Every loop requires several systemcalls beucase every file descriptor needs to be reregistered for every change in the polling events.
2. Regular files and others (like /dev/null) cannot be registered with epoll.

Problem 1. can be solved by using 4 epoll devices, one with each combination of file descriptor and polling event that is required.  I didn't find any solution to problem 2.

So I moved to using the poll system call, using a wrap over the libc function (note that openbsd-netcat also uses poll).

## Current status

Currently tcp over ipv4 is supported in both connect and listen modes.

Performance is similar to openbsd-netcat.
