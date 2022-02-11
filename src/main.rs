use std::env;
use std::io::{stdin, BufRead, Write};
use std::process::exit;
use std::str::FromStr;
use std::time::Instant;

use clap::Arg;

pub mod lib;

fn parse<T: FromStr>(s: &str) -> Option<T> {
    if let Ok(v) = s.parse() {
        Some(v)
    } else {
        eprintln!("Failed to parse value {s:?}");
        None
    }
}
#[derive(Debug)]
enum InputValue {
    Count(Vec<lib::Cluster>),
    List(Vec<Vec<f64>>),
}
impl InputValue {
    fn is_empty(&self) -> bool {
        match self {
            Self::Count(count) => count.is_empty(),
            Self::List(l) => l.is_empty(),
        }
    }
}

fn input(_is_tty: bool, debug_performance: bool, multiline: bool) -> Option<InputValue> {
    #[cfg(feature = "pretty")]
    if _is_tty {
        use std::io::stdout;

        if multiline {
            print!("multiline > ");
        } else {
            print!("> ")
        }
        stdout().lock().flush().unwrap();
    }
    let mut s = String::new();

    let now = Instant::now();

    let values = if multiline {
        let mut values = Vec::with_capacity(8);
        let stdin = stdin();
        let stdin = stdin.lock().lines();
        let mut lines = 0;
        for line in stdin {
            lines += 1;
            let line = line.unwrap();
            if line.trim().is_empty() {
                break;
            }
            let mut current = Vec::with_capacity(2);
            for segment in line.split(',') {
                let f = parse(segment.trim());
                if let Some(f) = f {
                    current.push(f)
                }
            }
            values.push(current);
            #[cfg(feature = "pretty")]
            if _is_tty {
                use std::io::stdout;

                let next = values.len() + 1;
                print!("{next} > ");
                stdout().lock().flush().unwrap();
            }
        }
        if lines <= 1 {
            exit(0);
        }
        InputValue::List(values)
    } else {
        stdin().lock().read_line(&mut s).unwrap();

        if s.trim().is_empty() {
            exit(0);
        }

        let values: Vec<_> = s
            .split(',')
            .map(|s| s.split_whitespace())
            .flatten()
            .filter_map(|s| {
                Some(if let Some((v, count)) = s.split_once('x') {
                    let count = parse(count)?;
                    (parse(v)?, count)
                } else {
                    (parse(s)?, 1)
                })
            })
            .collect();
        InputValue::Count(values)
    };

    if values.is_empty() {
        eprintln!("Only invalid input. Try again.");
        return None;
    }

    if debug_performance {
        println!("Parsing took {}µs", now.elapsed().as_micros());
    }
    Some(values)
}

fn main() {
    let app = clap::app_from_crate!();

    let app = app
        .arg(Arg::new("debug-performance").short('p').long("debug-performance"))
        .arg(Arg::new("multiline")
            .short('l')
            .long("multiline")
            .help("Accept multiple lines as one input. Two consecutive newlines is treated as the series separator. When not doing regression analysis the second 'column' is the count of the first. Acts more like CSV.")
        )
        .subcommand(clap::App::new("regression")
            .about("Find a equation which describes the input data. Tries to automatically determine the process.")
        );

    let matches = app.get_matches();

    let debug_performance = env::var("DEBUG_PERFORMANCE").ok().map_or_else(
        || matches.is_present("debug-performance"),
        |s| !s.trim().is_empty(),
    );

    #[cfg(feature = "pretty")]
    let tty = atty::is(atty::Stream::Stdin);
    #[cfg(not(feature = "pretty"))]
    let tty = false;

    'main: loop {
        let input = if let Some(i) = input(tty, debug_performance, matches.is_present("multiline"))
        {
            i
        } else {
            continue;
        };

        let values = {
            match input {
                InputValue::Count(count) => lib::OwnedClusterList::new(count),
                InputValue::List(list) => {
                    let mut count = Vec::with_capacity(list.len());
                    for item in list {
                        if item.len() != 1 && item.len() != 2 {
                            eprintln!("Expected one or two values per line.");
                            continue 'main;
                        }
                        let first = item[0];
                        let second = item.get(1).map_or(1, |f| f.round() as usize);
                        count.push((first, second))
                    }
                    lib::OwnedClusterList::new(count)
                }
            }
        };
        let now = Instant::now();

        let mut values = values.borrow().optimize_values();

        if debug_performance {
            println!("Optimizing input took {}µs", now.elapsed().as_micros());
        }
        let now = Instant::now();

        let mean = lib::std_dev(values.borrow());

        if debug_performance {
            println!(
                "Standard deviation & mean took {}µs",
                now.elapsed().as_micros()
            );
        }
        let now = Instant::now();

        // Sort of clusters required.
        values.sort_unstable_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let median = lib::median(lib::ClusterList::new(&values));

        if debug_performance {
            println!("Median & quadrilles took {}µs", now.elapsed().as_micros());
        }

        println!(
            "Standard deviation: {}, mean: {}, median: {}{}{}",
            mean.standard_deviation,
            mean.mean,
            median.median,
            median
                .lower_quadrille
                .as_ref()
                .map_or("".into(), |quadrille| {
                    format!(", lower quadrille: {}", *quadrille)
                }),
            median
                .higher_quadrille
                .as_ref()
                .map_or("".into(), |quadrille| {
                    format!(", upper quadrille: {}", *quadrille)
                }),
        );
    }
}
