use std::io::{self, Read};

use anyhow::Result;

fn main() -> Result<()> {
    if std::env::args().any(|arg| arg == "-h" || arg == "--help") {
        println!("Usage: parse-diff < diff.patch");
        println!("Reads a git diff from stdin and writes parsed JSON to stdout.");
        return Ok(());
    }

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let lines = input.lines().collect::<Vec<_>>();
    let diff = review_to_md::diff::parse(&lines)?.into_static();

    serde_json::to_writer_pretty(io::stdout(), &diff)?;
    println!();

    Ok(())
}
