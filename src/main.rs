use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::value::RawValue;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::time::Duration;

#[derive(Parser)]
#[command(about = "Convert JSON array to JSONL")]
struct Args {
    /// Input file (stdin if not provided)
    input: Option<String>,

    /// Output file (stdout if not provided)
    #[arg(short, long)]
    o: Option<String>,

    /// Progress bar
    #[arg(short = 'p', long)]
    progress: bool,
}

struct BufReaderWithCount<R> {
    count: u64,
    rd: BufReader<R>,
}

impl<R: Read> BufReaderWithCount<R> {
    fn new(r: R) -> Self {
        Self {
            count: 0,
            rd: BufReader::with_capacity(256 * 1024, r),
        }
    }
}

impl<T: Read> Read for BufReaderWithCount<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.rd.read(buf)?;
        self.count += n as u64;
        Ok(n)
    }
}
impl<T: Read> BufRead for BufReaderWithCount<T> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        return self.rd.fill_buf();
    }

    fn consume(&mut self, n: usize) {
        self.count += n as u64;
        self.rd.consume(n)
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum SkipRes {
    KeepSkipping,
    ExpectValue,
    End,
}

#[derive(Debug)]
struct SkipState {
    at_beginning: bool,
}

impl SkipState {
    fn skip(&mut self, buf: &[u8]) -> (SkipRes, usize) {
        let mut i = 0;
        while i < buf.len() {
            let c = buf[i];
            i += 1;
            if c == b' ' || c == b'\t' || c == b'\n' {
                continue;
            } else if c == b'[' && self.at_beginning {
                self.at_beginning = false;
                return (SkipRes::ExpectValue, i);
            } else if c == b',' && !self.at_beginning {
                return (SkipRes::ExpectValue, i);
            } else if c == b']' && !self.at_beginning {
                return (SkipRes::End, i);
            } else {
                panic!("malformed json")
            }
        }
        return (SkipRes::KeepSkipping, buf.len());
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // progress bar
    let progress = if args.progress {
        let bar = ProgressBar::new_spinner();
        bar.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise} | {total_bytes} | {bytes_per_sec}] {spinner}",
            )
            .unwrap(),
        );
        bar.enable_steady_tick(Duration::from_millis(200));
        Some(bar)
    } else {
        None
    };

    let input: Box<dyn io::Read> = match args.input {
        Some(path) => Box::new(File::open(path)?),
        None => Box::new(io::stdin().lock()),
    };

    let mut output: Box<dyn Write> = match args.o {
        Some(path) => Box::new(BufWriter::new(File::create(path)?)),
        None => Box::new(BufWriter::new(io::stdout().lock())),
    };

    let mut reader = BufReaderWithCount::new(input);
    let mut skip_st = SkipState { at_beginning: true };

    let mut old_count: u64 = 0;
    'outer_loop: loop {
        // remove leading '[' or ','
        'remove_prefix: loop {
            let (st, n) = {
                let buf = reader.fill_buf()?;
                skip_st.skip(buf)
            };

            reader.consume(n);

            match st {
                SkipRes::KeepSkipping => (),
                SkipRes::ExpectValue => {
                    break 'remove_prefix;
                }
                SkipRes::End => {
                    break 'outer_loop;
                }
            }
        }

        {
            let mut deser = serde_json::Deserializer::from_reader(&mut reader);
            let value: Box<RawValue> = serde::Deserialize::deserialize(&mut deser)?;
            serde_json::to_writer(&mut output, &value)?;
        }
        writeln!(output)?;

        let new_count = reader.count;
        if let Some(bar) = &progress {
            bar.inc(new_count - old_count);
        }

        old_count = new_count;
    }

    output.flush()?;

    if let Some(bar) = &progress {
        bar.finish();
    }
    Ok(())
}
