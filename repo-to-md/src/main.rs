use anyhow::Result;

use review_to_md::cli;

fn main() -> Result<()> {
    argh::from_env::<cli::Cli>().run()
}
