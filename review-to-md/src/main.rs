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

    /// review index to select (1-indexed, or -1 for last review)
    #[argh(option)]
    review_index: Option<i32>,

    /// filter reviews by author login
    #[argh(option)]
    author: Option<String>,
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
        // Interactive/indexed review selection flow
        let (owner, repo) = if let (Some(owner), Some(repo)) = (cli.owner, cli.repo) {
            (owner, repo)
        } else {
            get_repo_info()
                .context("Failed to auto-detect repository. Please provide --owner and --repo")?
        };

        // List all reviews for the PR
        let all_reviews = list_reviews(&owner, &repo, pr_id)?;

        // Apply author filter if specified
        let reviews: Vec<&Review> = if let Some(ref author) = cli.author {
            let filtered = filter_reviews_by_author(&all_reviews, author);
            if filtered.is_empty() {
                anyhow::bail!("No reviews found by author '{}' for PR #{}", author, pr_id);
            }
            eprintln!(
                "Filtered to {} review(s) by author '{}'",
                filtered.len(),
                author
            );
            filtered
        } else {
            all_reviews.iter().collect()
        };

        // Select review based on CLI options
        let selected_review = if let Some(index) = cli.review_index {
            // Direct selection by index
            select_review_by_index(&reviews, index)?
        } else if reviews.len() == 1 {
            // Auto-select if only one review (possibly filtered)
            eprintln!(
                "Auto-selecting only available review by {}",
                reviews[0].author.login
            );
            reviews[0]
        } else {
            // Interactive selection
            prompt_user_to_select_review(&reviews)?
        };

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
fn prompt_user_to_select_review<'a>(reviews: &[&'a Review]) -> Result<&'a Review> {
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
                return Ok(reviews[num - 1]);
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

/// Filters reviews by author login (case-insensitive).
///
/// # Arguments
///
/// * `reviews` - All reviews to filter
/// * `author` - The author login to filter by
///
/// # Returns
///
/// A vector of reviews by the specified author
pub(crate) fn filter_reviews_by_author<'a>(reviews: &'a [Review], author: &str) -> Vec<&'a Review> {
    reviews
        .iter()
        .filter(|r| r.author.login.eq_ignore_ascii_case(author))
        .collect()
}

/// Selects a review by index (1-indexed, or -1 for last).
///
/// # Arguments
///
/// * `reviews` - Reviews to select from
/// * `index` - 1-indexed position (or -1 for last)
///
/// # Returns
///
/// The selected review
///
/// # Errors
///
/// Returns an error if the index is out of bounds or invalid
pub(crate) fn select_review_by_index<'a>(reviews: &[&'a Review], index: i32) -> Result<&'a Review> {
    if reviews.is_empty() {
        anyhow::bail!("No reviews available to select from");
    }

    let selected = if index == -1 {
        *reviews.last().unwrap()
    } else if index >= 1 && index as usize <= reviews.len() {
        reviews[(index - 1) as usize]
    } else {
        anyhow::bail!(
            "Review index {} is out of bounds. Valid range: 1-{} or -1 for last review",
            index,
            reviews.len()
        );
    };

    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_review(id: &str, author: &str, comment_count: u32) -> Review {
        Review {
            id: id.to_string(),
            author: ReviewAuthor {
                login: author.to_string(),
            },
            state: "APPROVED".to_string(),
            body: Some("Test review".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            comments: CommentCount {
                total_count: comment_count,
            },
        }
    }

    #[test]
    fn test_filter_by_author() {
        let reviews = vec![
            create_test_review("1", "alice", 5),
            create_test_review("2", "bob", 3),
            create_test_review("3", "alice", 2),
        ];

        let filtered = filter_reviews_by_author(&reviews, "alice");
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].author.login, "alice");
        assert_eq!(filtered[1].author.login, "alice");
    }

    #[test]
    fn test_filter_case_insensitive() {
        let reviews = vec![create_test_review("1", "Alice", 5)];
        let filtered = filter_reviews_by_author(&reviews, "alice");
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_no_matches() {
        let reviews = vec![create_test_review("1", "alice", 5)];
        let filtered = filter_reviews_by_author(&reviews, "bob");
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_select_by_index_positive() {
        let reviews = [create_test_review("1", "alice", 5),
            create_test_review("2", "bob", 3)];
        let review_refs: Vec<&Review> = reviews.iter().collect();

        let selected = select_review_by_index(&review_refs, 1).unwrap();
        assert_eq!(selected.author.login, "alice");

        let selected = select_review_by_index(&review_refs, 2).unwrap();
        assert_eq!(selected.author.login, "bob");
    }

    #[test]
    fn test_select_by_index_last() {
        let reviews = [create_test_review("1", "alice", 5),
            create_test_review("2", "bob", 3)];
        let review_refs: Vec<&Review> = reviews.iter().collect();

        let selected = select_review_by_index(&review_refs, -1).unwrap();
        assert_eq!(selected.author.login, "bob");
    }

    #[test]
    fn test_select_out_of_bounds() {
        let reviews = [create_test_review("1", "alice", 5)];
        let review_refs: Vec<&Review> = reviews.iter().collect();

        assert!(select_review_by_index(&review_refs, 0).is_err());
        assert!(select_review_by_index(&review_refs, 2).is_err());
    }

    #[test]
    fn test_select_empty() {
        let reviews: Vec<Review> = vec![];
        let review_refs: Vec<&Review> = reviews.iter().collect();

        assert!(select_review_by_index(&review_refs, 1).is_err());
    }
}
