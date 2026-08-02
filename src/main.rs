use natsort_core::{compare, Ns};
use std::cmp::Ordering;
use std::io::{self, Read};

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
            other => {
                eprintln!("warning: ignoring unrecognized flag '{}'", other);
            }
        }
    }
    ns
}

fn print_usage() {
    eprintln!(
        "usage: natsort_port <sort|compare> [--real] [--float] [--signed] \\\n       [--ignorecase] [--lowercasefirst] [--groupletters]\n\n\
         sort:    reads lines from stdin, prints them naturally sorted\n\
         compare: reads exactly two lines from stdin, prints '<', '=', or '>'"
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        std::process::exit(2);
    }

    let cmd = args[0].as_str();
    let ns = parse_flags(&args[1..]);

    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("error: failed to read stdin");
        std::process::exit(1);
    }
    let lines: Vec<&str> = input.lines().collect();

    match cmd {
        "sort" => {
            let mut sorted: Vec<&str> = lines.clone();
            sorted.sort_by(|a, b| compare(a, b, ns));
            for line in sorted {
                println!("{}", line);
            }
        }
        "compare" => {
            if lines.len() != 2 {
                eprintln!(
                    "error: compare requires exactly two lines on stdin, got {}",
                    lines.len()
                );
                std::process::exit(2);
            }
            let ord = compare(lines[0], lines[1], ns);
            let symbol = match ord {
                Ordering::Less => "<",
                Ordering::Equal => "=",
                Ordering::Greater => ">",
            };
            println!("{}", symbol);
        }
        other => {
            eprintln!("error: unknown subcommand '{}'", other);
            print_usage();
            std::process::exit(2);
        }
    }
}
