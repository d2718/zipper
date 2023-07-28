# zipper

## Notice

This tool has been added to a collection of tools:

[`d2718/softies`](https://github.com/d2718/softies)

Further development will occur there; this repository will be archived.

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

It should just `cargo build` normally.

```sh
$ cargo build --release && \
    strip target/release/zipper
```

## Operation

`zipper` reads commands from stdin, one per line, until a blank line or
EOF, executes those commands, and interleaves their output, line by line.

```text
$ zipper
/bin/ls -l *.png
identify *.png

-rw-r--r-- 1 dan dan   38507 Apr 30  2021 2021_wrev_form_final.png
2021_wrev_form.png PNG 1279x480 1279x480+0+0 8-bit sRGB 38271B 0.000u 0:00.000
-rw-r--r-- 1 dan dan   38271 Apr 30  2021 2021_wrev_form.png
2021_wrev_form_final.png PNG 800x480 800x480+0+0 8-bit sRGB 38507B 0.000u 0:00.001
-rw-r--r-- 1 dan dan   18625 Mar 11  2022 cfit.png
cfit.png PNG 350x202 350x202+0+0 8-bit sRGB 18625B 0.000u 0:00.000
-rw-r--r-- 1 dan dan 4872997 Jan 10  2022 cloudy.png
cloudy.png PNG 2880x2160 2880x2160+0+0 8-bit sRGB 4.64725MiB 0.000u 0:00.000
-rw-r--r-- 1 dan dan 5810766 Jan 10  2022 frosty_1020.png
frosty_1020.png PNG 2160x2160 2160x2160+0+0 8-bit sRGB 5.54158MiB 0.000u 0:00.000
-rw-r--r-- 1 dan dan 2107900 Jan 10  2022 frosty_1080.png
frosty_1080.png PNG 1080x1080 1080x1080+0+0 8-bit sRGB 2107900B 0.000u 0:00.000
-rw-r--r-- 1 dan dan   46075 Dec 31  2022 mastodon.png
mastodon.png PNG 1200x1200 1200x1200+0+0 8-bit sRGB 46075B 0.000u 0:00.000
-rw-r--r-- 1 dan dan  630999 Jan 11  2022 trans_blue.png
trans_blue.png PNG 2400x1200 2400x1200+0+0 8-bit sRGB 255c 630999B 0.000u 0:00.000
-rw-r--r-- 1 dan dan  630999 Jan 11  2022 trans_pink.png
trans_pink.png PNG 2400x1200 2400x1200+0+0 8-bit sRGB 255c 630999B 0.000u 0:00.000
```

```text
$ export CPU=/sys/devices/system/cpu/cpu[0-3]/cpufreq
$ zipper
cat $CPU/scaling_cur_freq
cat $CPU/scaling_max_freq
cat $CPU/scaling_governor

798157
3300000
conservative
800000
3300000
conservative
798165
3300000
conservative
798261
3300000
conservative
```

It will take any number of commands, and by default will iterate through
them, line by line, until one exits, and then will stop.

```text
$ zipper
ls
ping 8.8.4.4
yes

2021_wrev_form_final.png
PING 8.8.4.4 (8.8.4.4) 56(84) bytes of data.
y
2021_wrev_form.png
64 bytes from 8.8.4.4: icmp_seq=1 ttl=116 time=17.7 ms
y
cfit.png
64 bytes from 8.8.4.4: icmp_seq=2 ttl=116 time=17.9 ms
y
cloudy.png
64 bytes from 8.8.4.4: icmp_seq=3 ttl=116 time=18.3 ms
y
frosty_1020.png
64 bytes from 8.8.4.4: icmp_seq=4 ttl=116 time=18.7 ms
y
frosty_1080.png
64 bytes from 8.8.4.4: icmp_seq=5 ttl=116 time=18.1 ms
y
mastodon.png
64 bytes from 8.8.4.4: icmp_seq=6 ttl=116 time=28.1 ms
y
rando
64 bytes from 8.8.4.4: icmp_seq=7 ttl=116 time=32.0 ms
y
trans_blue.png
64 bytes from 8.8.4.4: icmp_seq=8 ttl=116 time=29.0 ms
y
trans_pink.png
64 bytes from 8.8.4.4: icmp_seq=9 ttl=116 time=17.8 ms
y
```

The `-e ignore` or `-e blank` will cause it to continue until the last
command has exited, ignoring the terminated commands or inserting
blank lines where their output would otherwise be.

```text
$ zipper -e blank
cat pilots.txt
cat stokers.txt

William
Stephanie
Fred
Walter
Lisa
Harry T.
Helmut
Joe
Grace

Colin

Harry J.

Murph

```

You can control the behavior of individual commands by prepending some
arguments and separating them from the command with ` || ` (a double
pipe with whitespace on either side). `-s N` will skip `N` lines of
output; `-t N` will take only `N` lines of output.

```text
$ zipper -e ignore
-s 1 || ps
-t 7 || yes

 331791 pts/1    00:00:00 bash
y
 346381 pts/1    00:00:00 zipper
y
 346390 pts/1    00:00:00 sh
y
 346391 pts/1    00:00:00 sh
y
 346392 pts/1    00:00:00 ps
y
 346393 pts/1    00:00:00 yes
y
y
```

Finally, the `-d` command flag allows you to specify a custom
regular expression to use as a delimiter (instead of the default
newline).

```text
$ printf " -d [[:space:]] || cat /proc/version" | zipper
Linux
version
6.1.0-10-amd64
(debian-kernel@lists.debian.org)
(gcc-12
(Debian
12.2.0-14)
12.2.0,
GNU
ld
(GNU
Binutils
for
Debian)
2.40)
#1
SMP
PREEMPT_DYNAMIC
Debian
6.1.37-1
(2023-07-03)
```

## Big But

(with one 't')

This only works on systems with a POSIX-style `sh` shell.

This might change one day, but these kinds of pipe-oriented
command-line tools are way less useful on Windows.