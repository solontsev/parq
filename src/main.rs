use clap::Parser;
use parq::ParquetFileInfo;
use std::env;

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(version)]
#[command(about = env!("CARGO_PKG_DESCRIPTION"))]
#[command(author = env!("CARGO_PKG_AUTHORS"))]
struct Args {
    /// Path to the parquet file to analyze
    #[arg(value_name = "FILE")]
    filename: String,
}

fn main() {
    let args = Args::parse();

    match ParquetFileInfo::new(&args.filename) {
        Ok(info) => println!("{info}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
