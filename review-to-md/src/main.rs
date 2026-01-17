use anyhow::{Context, Result};
use argh::FromArgs;
use review_to_md::*;
use std::io::{self, Write};

#[derive(FromArgs)]
/// Format GitHub PR comments as markdown
struct Cli {
    /// PR number to fetch comments from (not needed if --json-file is provided)
    #[argh(positional)]
    pr_id: Option<u32>,

    /// path to local JSON file containing saved API response
    #[argh(option)]
    json_file: Option<String>,

    /// repository owner (auto-detected from git remote if not provided)
    #[argh(option)]
    owner: Option<String>,

    /// repository name (auto-detected from git remote if not provided)
    #[argh(option)]
    repo: Option<String>,

    /// specific review ID to fetch comments from (skips interactive selection)
    #[argh(option)]
    review_id: Option<String>,
}

/// Prompts the user to select a review from a list.
///
/// Displays a table of reviews with their index, author, date, comment count,
/// and truncated body text. Reads user input to select a review by number.
///
/// # Arguments
///
/// * `reviews` - A vector of reviews to choose from
///
/// # Returns
///
/// The selected [`Review`]
///
/// # Errors
///
/// Returns an error if:
/// - No reviews are provided
/// - User input is invalid
/// - User enters 'q' to quit
fn prompt_user_to_select_review(reviews: &[Review]) -> Result<&Review> {
    if reviews.is_empty() {
        anyhow::bail!("No reviews found for this pull request");
    }

    // Display header
    eprintln!("\nAvailable reviews for this PR:\n");
    eprintln!(
        "{:<4} {:<15} {:<20} {:<8} Body",
        "No.", "Author", "Date", "Comments"
    );
    eprintln!("{}", "-".repeat(80));

    // Display each review
    for (i, review) in reviews.iter().enumerate() {
        let body_preview = review
            .body
            .as_ref()
            .map(|b| {
                let truncated = if b.len() > 50 {
                    format!("{}...", &b[..47])
                } else {
                    b.clone()
                };
                // Replace newlines with spaces for display
                truncated.replace('\n', " ")
            })
            .unwrap_or_else(|| "(no description)".to_string());

        // Extract just the date part (YYYY-MM-DD)
        let date = review.created_at.split('T').next().unwrap_or("");

        eprintln!(
            "{:<4} {:<15} {:<20} {:<8} {}",
            i + 1,
            review.author.login,
            date,
            review.comments.total_count,
            body_preview
        );
    }

    eprintln!();

    // Prompt for selection
    loop {
        eprint!("Select review (1-{}, or 'q' to quit): ", reviews.len());
        io::stderr().flush()?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .context("Failed to read user input")?;

        let input = input.trim();

        // Check for quit
        if input.eq_ignore_ascii_case("q") {
            anyhow::bail!("User cancelled review selection");
        }

        // Try to parse as number
        match input.parse::<usize>() {
            Ok(num) if num >= 1 && num <= reviews.len() => {
                return Ok(&reviews[num - 1]);
            }
            _ => {
                eprintln!(
                    "Invalid selection. Please enter a number between 1 and {}, or 'q' to quit.",
                    reviews.len()
                );
            }
        }
    }
}

fn main() -> Result<()> {
    let cli: Cli = argh::from_env();

    let comments = if let Some(json_file) = cli.json_file {
        // Read from local JSON file
        read_comments_from_file(&json_file)?
    } else if let Some(review_id) = cli.review_id {
        // Fetch comments from specific review (skip interactive selection)
        fetch_review_comments(&review_id)?
    } else if let Some(pr_id) = cli.pr_id {
        // Interactive review selection flow
        let (owner, repo) = if let (Some(owner), Some(repo)) = (cli.owner, cli.repo) {
            (owner, repo)
        } else {
            get_repo_info()
                .context("Failed to auto-detect repository. Please provide --owner and --repo")?
        };

        // List all reviews for the PR
        let reviews = list_reviews(&owner, &repo, pr_id)?;

        // Prompt user to select a review
        let selected_review = prompt_user_to_select_review(&reviews)?;

        eprintln!(
            "\nFetching comments from review by {}...\n",
            selected_review.author.login
        );

        // Fetch comments from the selected review
        fetch_review_comments(&selected_review.id)?
    } else {
        anyhow::bail!("Either provide PR_ID or --json-file");
    };

    if comments.is_empty() {
        eprintln!("No comments found");
        return Ok(());
    }

    let grouped_comments = group_comments_by_file(comments);
    let markdown = format_comments_as_markdown(grouped_comments);
    print!("{}", markdown);

    Ok(())
}
