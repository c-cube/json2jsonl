use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
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
    count: Arc<AtomicU64>,
    rd: BufReader<R>,
}

impl<R: Read> BufReaderWithCount<R> {
    fn new(r: R) -> Self {
        Self {
            count: Arc::new(AtomicU64::new(0)),
            rd: BufReader::new(r),
        }
    }
}

impl<T: Read> Read for BufReaderWithCount<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.rd.read(buf)?;
        self.count.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }
}
impl<T: Read> BufRead for BufReaderWithCount<T> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        return self.rd.fill_buf();
    }

    fn consume(&mut self, n: usize) {
        self.count.fetch_add(n as u64, Ordering::Relaxed);
        self.rd.consume(n)
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

    let reader = BufReaderWithCount::new(input);
    let reader_count = reader.count.clone();
    let stream = large_json_array::JsonStream::new(reader);

    let mut old_count = 0;
    for value in stream {
        let value = value?;
        serde_json::to_writer(&mut output, &value)?;
        writeln!(output)?;

        let new_count = reader_count.load(Ordering::Relaxed);

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
