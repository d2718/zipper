# zipper
A command-line utility to interleave output.

```text
Interleave the outputs of multiple commands.

Usage: zipper [OPTIONS]

Options:
  -e, --exit <EXIT>
          Specify behavior on command termination
          
          [default: terminate]

          Possible values:
          - terminate: Stop when first command terminates
          - ignore:    Ignore terminated commands
          - blank:     Insert blank lines for commands that have terminated

  -t, --threads
          Use more threads

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## Installation

Because the [`regex-chunker`](https://github.com/d2718/regex-chunker/tree/master)
dependency requires the nightly compiler, this also currently requires the nightly
compiler to build. Once `regex-chunker` gets that figured out, this should build
normally.

```sh
$ cargo +nightly build --release && \
    strip target/release/zipper
```

## Operation

`zipper` reads commands from stdin, one per line, until a blank line or
EOF, executes those commands, and interleaves their output, line by line.

```text
$ zipper
lsb_release -a
ping 8.8.4.4

Distributor ID:	Debian
PING 8.8.4.4 (8.8.4.4) 56(84) bytes of data.
Description:	Debian GNU/Linux 12 (bookworm)
64 bytes from 8.8.4.4: icmp_seq=1 ttl=116 time=18.1 ms
Release:	12
64 bytes from 8.8.4.4: icmp_seq=2 ttl=116 time=17.9 ms
Codename:	bookworm
64 bytes from 8.8.4.4: icmp_seq=3 ttl=116 time=18.1 ms
```

It will take any number of commands, and by default will iterate through
them, line by line, until one exits, and then will stop.

