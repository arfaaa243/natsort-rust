//! `natsort_port` — a small CLI over the `natsort_core` library.
//!
//! # Exit codes
//! - `0` — success.
//! - `1` — I/O error (a given input file couldn't be read, or stdin
//!   couldn't be read).
//! - `2` — usage error (no arguments, unknown subcommand, unrecognized
//!   flag, wrong number of lines for `compare`, or an unexpected extra
//!   positional argument).

use natsort_core::{compare, natsort_keygen, Ns};
use std::cmp::Ordering;
use std::io::{self, Read};

const EXIT_IO_ERROR: i32 = 1;
const EXIT_USAGE_ERROR: i32 = 2;

/// The `--real`/`--float`/... family of algorithm flags this CLI
/// recognizes. Kept as a lookup table so unrecognized `--flags` can be
/// rejected explicitly (a usage error) rather than silently ignored.
const KNOWN_ALGO_FLAGS: &[&str] = &[
    "--real",
    "--float",
    "--signed",
    "--ignorecase",
    "--lowercasefirst",
    "--groupletters",
];

fn parse_flags(args: &[String]) -> Ns {
    let mut ns = Ns::DEFAULT;
    for a in args {
        match a.as_str() {
            "--real" => {
                ns.float = true;
                ns.signed = true;
            }
            "--float" => ns.float = true,
            "--signed" => ns.signed = true,
            "--ignorecase" => ns.ignorecase = true,
            "--lowercasefirst" => ns.lowercasefirst = true,
            "--groupletters" => ns.groupletters = true,
            // parse_flags only ever receives args already validated
            // against KNOWN_ALGO_FLAGS by main(), so this is unreachable
            // in practice; kept as a safe fallback rather than a panic.
            _ => {}
        }
    }
    ns
}

fn print_usage() {
    eprintln!(
        "usage: natsort_port <sort|compare> [--real] [--float] [--signed] \\\n       [--ignorecase] [--lowercasefirst] [--groupletters] [--reverse] [FILE]\n\n\
         sort:    reads lines from FILE (or stdin if omitted), prints them naturally sorted\n\
         compare: reads exactly two lines from FILE (or stdin), prints '<', '=', or '>'\n\n\
         --reverse           reverses the final sorted order (sort subcommand only)\n\
         --help, -h          print this message and exit 0\n\
         --version, -V       print the version and exit 0\n\n\
         exit codes: 0 success, 1 I/O error, 2 usage error"
    );
}

fn print_version() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}

fn read_input(path: Option<&str>) -> Result<String, String> {
    match path {
        Some(p) => std::fs::read_to_string(p).map_err(|e| format!("failed to read '{p}': {e}")),
        None => {
            let mut s = String::new();
            io::stdin()
                .read_to_string(&mut s)
                .map_err(|e| format!("failed to read stdin: {e}"))?;
            Ok(s)
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        print_usage();
        std::process::exit(EXIT_USAGE_ERROR);
    }

    if args[0] == "--help" || args[0] == "-h" {
        print_usage();
        return;
    }
    if args[0] == "--version" || args[0] == "-V" {
        print_version();
        return;
    }

    let cmd = args[0].as_str();
    if cmd != "sort" && cmd != "compare" {
        eprintln!("error: unknown subcommand '{cmd}'");
        print_usage();
        std::process::exit(EXIT_USAGE_ERROR);
    }

    // Split the remaining args into: the --reverse flag (CLI-only output
    // transform, not part of the natsort algorithm itself — see
    // DECISIONS.md #11), recognized algorithm flags, unrecognized flags
    // (a hard usage error, not a silent warning — see DECISIONS.md #15),
    // and at most one positional argument (an input file path).
    let mut reverse = false;
    let mut algo_flags: Vec<String> = Vec::new();
    let mut unknown_flags: Vec<String> = Vec::new();
    let mut input_path: Option<String> = None;

    for a in &args[1..] {
        if a == "--reverse" {
            reverse = true;
        } else if a.starts_with("--") {
            if KNOWN_ALGO_FLAGS.contains(&a.as_str()) {
                algo_flags.push(a.clone());
            } else {
                unknown_flags.push(a.clone());
            }
        } else if input_path.is_some() {
            eprintln!("error: unexpected extra argument '{a}'");
            print_usage();
            std::process::exit(EXIT_USAGE_ERROR);
        } else {
            input_path = Some(a.clone());
        }
    }

    if !unknown_flags.is_empty() {
        for f in &unknown_flags {
            eprintln!("error: unrecognized flag '{f}'");
        }
        print_usage();
        std::process::exit(EXIT_USAGE_ERROR);
    }

    let ns = parse_flags(&algo_flags);

    let input = match read_input(input_path.as_deref()) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            std::process::exit(EXIT_IO_ERROR);
        }
    };
    let lines: Vec<&str> = input.lines().collect();

    match cmd {
        "sort" => {
            // Use the reusable keygen + sort_by_cached_key so each line's
            // key is parsed exactly once, not on every comparison.
            let keygen = natsort_keygen(ns);
            let mut sorted: Vec<&str> = lines.clone();
            sorted.sort_by_cached_key(|s| keygen(s));
            if reverse {
                sorted.reverse();
            }
            for line in sorted {
                println!("{line}");
            }
        }
        "compare" => {
            if lines.len() != 2 {
                eprintln!(
                    "error: compare requires exactly two lines on stdin, got {}",
                    lines.len()
                );
                std::process::exit(EXIT_USAGE_ERROR);
            }
            let ord = compare(lines[0], lines[1], ns);
            let symbol = match ord {
                Ordering::Less => "<",
                Ordering::Equal => "=",
                Ordering::Greater => ">",
            };
            println!("{symbol}");
        }
        // Unreachable: `cmd` was already validated to be "sort" or
        // "compare" above.
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_flags_real_implies_float_and_signed() {
        let ns = parse_flags(&["--real".to_string()]);
        assert!(ns.float);
        assert!(ns.signed);
    }

    #[test]
    fn parse_flags_default_is_all_false() {
        let ns = parse_flags(&[]);
        assert_eq!(ns, Ns::DEFAULT);
    }

    #[test]
    fn parse_flags_combines_multiple_flags() {
        let ns = parse_flags(&["--signed".to_string(), "--ignorecase".to_string()]);
        assert!(ns.signed);
        assert!(ns.ignorecase);
        assert!(!ns.float);
    }

    #[test]
    fn known_algo_flags_table_matches_parse_flags_cases() {
        // Every flag parse_flags gives real behavior to must also be in
        // KNOWN_ALGO_FLAGS, or main() would reject it before parse_flags
        // ever saw it.
        for &flag in &["--real", "--float", "--signed", "--ignorecase", "--lowercasefirst", "--groupletters"] {
            assert!(
                KNOWN_ALGO_FLAGS.contains(&flag),
                "'{flag}' is handled by parse_flags but missing from KNOWN_ALGO_FLAGS"
            );
        }
    }

    #[test]
    fn read_input_from_missing_file_is_an_error() {
        let result = read_input(Some("/definitely/does/not/exist/natsort_port_test.txt"));
        assert!(result.is_err());
    }
}
