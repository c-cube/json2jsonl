use clap::Parser;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};

#[derive(Parser)]
#[command(about = "Convert JSON array to JSONL")]
struct Args {
    /// Input file (stdin if not provided)
    input: Option<String>,

    /// Output file (stdout if not provided)
    #[arg(short, long)]
    o: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let input: Box<dyn io::Read> = match args.input {
        Some(path) => Box::new(File::open(path)?),
        None => Box::new(io::stdin().lock()),
    };

    let mut output: Box<dyn Write> = match args.o {
        Some(path) => Box::new(BufWriter::new(File::create(path)?)),
        None => Box::new(io::stdout().lock()),
    };

    let reader = BufReader::new(input);
    let stream = large_json_array::JsonStream::new(reader);

    for value in stream {
        let value = value?;
        serde_json::to_writer(&mut output, &value)?;
        writeln!(output)?;
    }

    output.flush()?;
    Ok(())
}
