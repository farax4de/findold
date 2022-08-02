# findold

A small tool to find old files. Found files can be sent to the linux syslog (facility damon)
as warning or error - or be output to stdout/stderr.


## usage:
```
USAGE:
    findold [OPTIONS] <PATH> [--] [FILE TIME ATTRIBUTE]

ARGS:
    <PATH>                   start the search from this path
    <FILE TIME ATTRIBUTE>    file timestamp attribute for comparison [default: ctime] [possible
                             values: atime, mtime, ctime]

OPTIONS:
    -h, --help
            Print help information

    -o, --output-target <OUTPUT TARGET>...
            warn and error writes to syslog facility daemon, csv writes to stdout as csv lines with
            delimiter ";", csv columns: ["TIME ATTRIBUTE";TIME OFFSET(sec);"FILENAME";FILE AGE(sec)]
            [default: stdout] [possible values: warn, error, stderr, stdout, csv]

    -r, --regex <REGUALR EXPRESSION>
            filter stripd path by regular expression pattern - use hard quotes!

    -t, --time-offset <TIME OFFSET>
            offset relative back to the current time, value in seconds, minutes or hours (eg: 10s,
            5m or 5h) if an suffix is missing, then second is used [default: 0]

    -V, --version
            Print version information
```
