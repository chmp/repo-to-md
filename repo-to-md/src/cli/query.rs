use std::io::Write;

use anyhow::{Result, bail, ensure};
use argh::FromArgs;

use crate::client::{
    FetchReviewCommentsClient, GetCurrentUserClient, ListPullRequestsClient, ListReviewsClient,
};

#[derive(FromArgs)]
#[argh(subcommand, name = ":query")]
/// Perform one of the internal queries used for debugging dev
pub struct QueryCommand {
    /// the command as a string spec
    #[argh(positional)]
    pub query: String,

    #[argh(positional)]
    // the arguments of the query
    pub args: Vec<String>,
}

impl QueryCommand {
    pub fn run(
        self,
        client: &(
             impl GetCurrentUserClient
             + FetchReviewCommentsClient
             + ListReviewsClient
             + ListPullRequestsClient
         ),
    ) -> Result<()> {
        match self.query.as_str() {
            "GetCurrentUser" => {
                ensure!(
                    self.args.is_empty(),
                    "GetCurrentUser does not expect any arguments"
                );
                serde_json::to_writer(std::io::stdout(), &client.get_current_user()?)?;
            }
            "ListReviews" => {
                let Ok([owner, repo, pr_number]) = <[_; 3]>::try_from(self.args) else {
                    bail!("ListRevies should be called with owner, repo, pr_number");
                };
                let pr_number: u32 = pr_number.parse()?;
                let result = client.list_reviews(&owner, &repo, pr_number)?;
                serde_json::to_writer(std::io::stdout(), &result)?;
            }
            "FetchReviewComments" => {
                let Ok([review_id]) = <[_; 1]>::try_from(self.args) else {
                    bail!("FetchReviewComments should be called with the review_id");
                };
                let result = client.fetch_review_comments(&review_id)?;
                serde_json::to_writer(std::io::stdout(), &result)?;
            }
            "ListPullRequests" => {
                let Ok([owner, repo]) = <[_; 2]>::try_from(self.args) else {
                    bail!("ListPullRequests should be called with owner, repo");
                };
                let result = client.list_pull_requests(&owner, &repo)?;
                serde_json::to_writer(std::io::stdout(), &result)?;
            }
            cmd => bail!("unkown command {cmd:?}"),
        }

        std::io::stdout().flush()?;

        Ok(())
    }
}
