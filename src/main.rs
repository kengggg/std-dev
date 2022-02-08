use std::collections::HashMap;
use std::io::{stdin, stdout, BufRead, Write};
use std::process::exit;
use std::str::FromStr;
use std::time::Instant;
use std::{env, hash};

fn f() {
    stdout().lock().flush().unwrap();
}

struct MultipleList<'a> {
    list: &'a [(f64, usize)],
}
impl<'a> MultipleList<'a> {
    fn new(list: &'a [(f64, usize)]) -> Self {
        Self { list }
    }

    fn len(&self) -> usize {
        self.list.iter().map(|(_, count)| *count).sum()
    }
    fn sum(&self) -> f64 {
        let mut sum = 0.0;
        for (v, count) in self.list.iter() {
            sum += v * *count as f64;
        }
        sum
    }
    fn sum_squared_diff(&self, base: f64) -> f64 {
        let mut sum = 0.0;
        for (v, count) in self.list.iter() {
            sum += (v - base).powi(2) * *count as f64;
        }
        sum
    }
    /// The inner list must be sorted by the `f64`.
    fn median(&self) -> f64 {
        let len = self.len();
        let even = len % 2 == 0;
        let mut len = len;
        let target = len / 2;

        for (pos, (v, count)) in self.list.iter().enumerate() {
            len -= *count;
            if len + 1 == target && even {
                let mean = (*v + self.list[pos - 1].0) / 2.0;
                return mean;
            }
            if len < target || len == target && !even {
                return *v;
            }
        }
        0.0
    }
    fn split_start(&self, len: usize) -> Vec<(f64, usize)> {
        let mut sum = 0;
        let mut list = Vec::new();
        for (v, count) in self.list {
            sum += count;
            if sum >= len {
                list.push((*v, *count - (sum - len)));
                break;
            } else {
                list.push((*v, *count));
            }
        }
        list
    }
    fn split_end(&self, len: usize) -> Vec<(f64, usize)> {
        let len = self.len() - len;
        let mut sum = self.len();
        let mut list = Vec::new();
        for (v, count) in self.list.iter().rev() {
            sum -= count;
            if sum <= len {
                list.insert(0, (*v, *count - (len - sum)));
                break;
            } else {
                list.insert(0, (*v, *count))
            }
        }
        list
    }
}

struct Output {
    s: f64,
    m: f64,
}
struct MedianOutput {
    median: f64,
    lower_quadrille: Option<f64>,
    higher_quadrille: Option<f64>,
}

fn std_dev(values: MultipleList) -> Output {
    let m = values.sum() / values.len() as f64;
    let squared_deviations = values.sum_squared_diff(m);
    let variance: f64 = squared_deviations / (values.len() - 1) as f64;
    Output {
        s: variance.sqrt(),
        m,
    }
}
fn median(values: MultipleList) -> MedianOutput {
    fn median(sorted_values: &[f64]) -> f64 {
        // even
        if sorted_values.len() % 2 == 0 {
            let b = sorted_values.len() / 2;
            let a = b - 1;
            (sorted_values[a] + sorted_values[b]) / 2.0
        } else {
            sorted_values[sorted_values.len() / 2]
        }
    }
    let lower_half = values.split_start(values.len() / 2);
    let lower_half = MultipleList::new(&lower_half);
    let upper_half = values.split_end(values.len() / 2);
    let upper_half = MultipleList::new(&upper_half);
    MedianOutput {
        median: values.median(),
        lower_quadrille: if lower_half.len() > 1 {
            Some(lower_half.median())
        } else {
            None
        },
        higher_quadrille: if upper_half.len() > 1 {
            Some(upper_half.median())
        } else {
            None
        },
    }
}

fn parse<T: FromStr>(s: &str) -> Option<T> {
    if let Ok(v) = s.parse() {
        Some(v)
    } else {
        eprintln!("Failed to parse value {s:?}");
        None
    }
}

#[derive(Debug, Copy, Clone)]
struct F64Hash(f64);

impl F64Hash {
    fn key(&self) -> u64 {
        self.0.to_bits()
    }
}

impl hash::Hash for F64Hash {
    fn hash<H>(&self, state: &mut H)
    where
        H: hash::Hasher,
    {
        self.key().hash(state)
    }
}

impl PartialEq for F64Hash {
    fn eq(&self, other: &F64Hash) -> bool {
        self.key() == other.key()
    }
}

impl Eq for F64Hash {}

fn main() {
    let performance_print = env::var("DEBUG_PERFORMANCE")
        .ok()
        .map_or(false, |s| !s.trim().is_empty());

    let tty = atty::is(atty::Stream::Stdin);

    loop {
        if tty {
            print!("> ");
            f();
        }
        let mut s = String::new();

        stdin().lock().read_line(&mut s).unwrap();

        if s.trim().is_empty() {
            exit(0);
        }

        let now = Instant::now();

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

        if values.is_empty() {
            eprintln!("Only invalid input. Try again.");
            continue;
        }

        if performance_print {
            println!("Parsing took {}µs", now.elapsed().as_micros());
        }
        let now = Instant::now();

        let mut collected = HashMap::with_capacity(16);
        for (v, count) in &values {
            let c = collected.entry(F64Hash(*v)).or_insert(0);
            *c += count;
        }
        let mut values: Vec<_> = collected.into_iter().map(|(f, c)| (f.0, c)).collect();

        if performance_print {
            println!("Optimizing input took {}µs", now.elapsed().as_micros());
        }
        let now = Instant::now();

        let out = std_dev(MultipleList::new(&values));

        if performance_print {
            println!(
                "Standard deviation & mean took {}µs",
                now.elapsed().as_micros()
            );
        }
        let now = Instant::now();

        values.sort_unstable_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let median = median(MultipleList::new(&values));

        if performance_print {
            println!("Median & quadrilles took {}µs", now.elapsed().as_micros());
        }

        println!(
            "Standard deviation: {}, mean: {}, median: {}, lower quadrille: {:?}, higher quadrille: {:?}",
            out.s, out.m, median.median, median.lower_quadrille, median.higher_quadrille,
        );
    }
}
