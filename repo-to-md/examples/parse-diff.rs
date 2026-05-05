use std::io::{self, Read};

use anyhow::Result;

fn main() -> Result<()> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let lines = input.lines().collect::<Vec<_>>();
    let diff = review_to_md::diff_v2::parse(&lines)?.into_static();

    serde_json::to_writer_pretty(io::stdout(), &diff)?;
    println!();

    Ok(())
}
