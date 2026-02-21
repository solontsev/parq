use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(version)]
#[command(about = env!("CARGO_PKG_DESCRIPTION"))]
#[command(author = env!("CARGO_PKG_AUTHORS"))]
pub struct Args {
    /// Path to the parquet file to analyze
    #[arg(value_name = "FILE")]
    pub filename: String,

    /// Maximum number of characters to display for metadata values before truncating
    #[arg(short = 'l', long, default_value_t = 300)]
    pub max_value_length: usize,

    /// Disable truncation of metadata values
    #[arg(long)]
    pub no_truncate: bool,
}
