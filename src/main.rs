extern crate chrono;
extern crate clap;
extern crate regex;
extern crate std;
extern crate syslog;
extern crate walkdir;
#[macro_use]
extern crate log;

use clap::{Arg, Command};
use fancy_regex::Regex;
use humantime::format_duration;
use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::string::String;
use std::time::Duration;
use std::time::SystemTime;
use std::{fs, println};
use syslog::{Error, Facility};
use walkdir::WalkDir;

const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
const AUTHORS: Option<&'static str> = option_env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: Option<&'static str> = option_env!("CARGO_PKG_DESCRIPTION");

// get time difference
fn elapsed_seconds(start: SystemTime, end: SystemTime) -> u64 {
    match start.duration_since(end) {
        Ok(n) => n.as_secs(),
        Err(e) => {
            eprintln!("ERROR in fkt. \"elapsed_seconds()\" - {:?} ", e);
            std::process::exit(1)
        }
    }
}

fn prog() -> Option<String> {
    env::args()
        .next()
        .as_ref()
        .map(Path::new)
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .map(String::from)
}

// parse usize from str or exit with error
fn get_numeric_from_string(num_text: &str) -> usize {
    match num_text.parse::<usize>() {
        Ok(val) => {
            return val;
        }
        Err(e) => {
            eprintln!(
                "ERROR in fkt. \"get_numeric_from_string()\" value {:?} is not numeric - {:?} ",
                num_text, e
            );
            std::process::exit(1)
        }
    };
}

// get seconds from time string
// if last char is in ['s', 'S', 'm', 'M'] then calculate as seconds or minutes.
fn get_seconds(timestring: &str) -> usize {
    let timestring = timestring.to_string();
    match timestring.chars().last() {
        Some(postfix) => match postfix {
            's' | 'S' => return get_numeric_from_string(&timestring[0..timestring.len() - 1]),
            'm' | 'M' => return get_numeric_from_string(&timestring[0..timestring.len() - 1]) * 60,
            'h' | 'H' => {
                return get_numeric_from_string(&timestring[0..timestring.len() - 1]) * 3600
            }
            'd' | 'D' => {
                return get_numeric_from_string(&timestring[0..timestring.len() - 1]) * 86400
            }
            _ => return get_numeric_from_string(&timestring),
        },
        None => {}
    }
    return 0;
}

// get Regex from Opion<&str>
fn get_regex(regex_string: Option<&str>) -> Option<fancy_regex::Regex> {
    match regex_string {
        Some(s) => match Regex::new(s) {
            Ok(r) => return Some(r),
            Err(e) => {
                eprintln!(
                    "ERROR in fkt. \"get_regex()\" value {:?} is not numeric - {:?} ",
                    regex_string, e
                );
                std::process::exit(1)
            }
        },
        None => return None,
    }
}

fn write_to_out(
    out_target: &str,
    time_attr: &str,
    time_offset: &Duration,
    time_diff: &Duration,
    filename: &Path,
) {
    match out_target {
        "csv" => {
            println!(
                "\"{}\";{};\"{}\";{}",
                time_attr,
                time_offset.as_secs(),
                filename.display(),
                time_diff.as_secs(),
            )
        }
        "stdout" => {
            println!(
                "file({}) \"{}\" is older than {} - the age is: {}",
                time_attr,
                filename.display(),
                format_duration(*time_offset),
                format_duration(*time_diff)
            )
        }
        "stderr" => {
            eprintln!(
                "file({}) \"{}\" is older than {} - the age is: {}",
                time_attr,
                filename.display(),
                format_duration(*time_offset),
                format_duration(*time_diff)
            )
        }
        "warn" => {
            warn!(
                "file({}) {} is older than {}",
                time_attr,
                filename.display(),
                format_duration(*time_offset)
            )
        }
        "error" => {
            error!(
                "file({}) {} is older than {}",
                time_attr,
                filename.display(),
                format_duration(*time_offset)
            )
        }
        _ => {
            println!(
                "{} older than {}] - {} [age:{}]",
                time_attr,
                time_offset.as_secs(),
                filename.display(),
                time_diff.as_secs()
            )
        }
    }
}

// get timestamp from file by attribute type (atime, ctime, mtime)
fn get_timestamp_by_attr(
    fs_path: &walkdir::DirEntry,
    attr: &str,
) -> Result<SystemTime, std::io::Error> {
    let f_metadata = fs::metadata(fs_path.path()).unwrap();
    match attr {
        "atime" => return f_metadata.accessed(),
        "ctime" => return f_metadata.created(),
        "mtime" => {
            return f_metadata.modified();
        }
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "oops, time attribute not found",
        )),
    }
}

// call args: start path, minutes offset
fn main() -> Result<(), Error> {
    let this_file = prog();
    syslog::init(
        Facility::LOG_DAEMON,
        log::LevelFilter::Debug,
        Some(&this_file.unwrap()),
    )?;
    let out_target_values = ["warn", "error", "stderr", "stdout", "csv"];

    let matches = Command::new("findold")
        .version(VERSION.unwrap_or("unknown"))
        .author(AUTHORS.unwrap_or("unknown"))
        .about(DESCRIPTION.unwrap_or("unknown"))
        .arg(Arg::new("start_path")
            .value_name("PATH")
            .help("start the search from this path")
            .required(true)
            .takes_value(true))
        .arg(Arg::new("time_attribute")
            .value_name("FILE TIME ATTRIBUTE")
            .help("file timestamp attribute for comparison")
            // .required(true)
            .default_value("ctime")
            .possible_values(["atime", "mtime", "ctime"])
            .takes_value(true))
        .arg(Arg::new("time_offset")
            .help("offset relative back to the current time, value in seconds, minutes or hours (eg: 10s, 5m or 5h) \
                if an suffix is missing, then second is used")
            .value_name("TIME OFFSET")
            .short('t')
            .long("time-offset")
            .default_value("0")
            .required(false))
        .arg(Arg::new("output_targets")
            .help("warn and error writes to syslog facility daemon, \
                csv writes to stdout as csv lines with delimiter \";\", \
                csv columns: [\"TIME ATTRIBUTE\";TIME OFFSET(sec);\"FILENAME\";FILE AGE(sec)]")
            .value_name("OUTPUT TARGET")
            .short('o')
            .long("output-target")
            .default_value("stdout")
            .multiple(true)
            .possible_values(&out_target_values)
            .required(false))
        .arg(Arg::new("regex")
            .value_name("REGUALR EXPRESSION")
            .help("filter stripd path by regular expression pattern - \
                    use hard quotes!")
            .short('r')
            .long("regex")
            .required(false)
            .takes_value(true)
        ).get_matches();

    let start_path = matches.value_of("start_path").unwrap();
    let regex = get_regex(matches.value_of("regex"));
    let time_attr = matches.value_of("time_attribute").unwrap();
    let time_offset = get_seconds(matches.value_of("time_offset").unwrap());
    let mut out_targets: Vec<_> = matches.values_of("output_targets").unwrap().collect();
    out_targets.dedup_by_key(|a| a.to_uppercase());
    let walker = WalkDir::new(start_path).into_iter();
    let systime_now = SystemTime::now();
    let t_offset = Duration::new(
        get_seconds(matches.value_of("time_offset").unwrap())
            .try_into()
            .unwrap(),
        0.try_into().unwrap(),
    );

    for entry in walker {
        match entry {
            Err(err) => {
                info!("INFO, no {:?}", err);
                continue;
            }
            _ => (),
        }

        let entry = entry.expect("cant open dir");

        if entry.file_type().is_file() {
            let age_in_sec = Duration::new(
                elapsed_seconds(
                    systime_now,
                    get_timestamp_by_attr(&entry, &time_attr).unwrap(),
                ),
                0,
            );
            if age_in_sec.as_secs() > time_offset.try_into().unwrap() {
                let fq_path = PathBuf::from(&entry.path());
                let striped_path = fq_path.strip_prefix(start_path).unwrap();
                match &regex {
                    Some(rx) => {
                        if rx.is_match(striped_path.to_str().unwrap()).unwrap() {
                            for target in &out_targets {
                                write_to_out(
                                    &target,
                                    &time_attr,
                                    &t_offset,
                                    &age_in_sec,
                                    striped_path,
                                )
                            }
                        }
                    }
                    None => {
                        for target in &out_targets {
                            write_to_out(&target, &time_attr, &t_offset, &age_in_sec, striped_path)
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
