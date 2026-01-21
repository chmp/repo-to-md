use std::collections::HashMap;

use anyhow::{anyhow, Result};

use crate::client::{
    Comment, FetchReviewCommentsClient, GetCurrentUserClient, ListReviewsClient, Review, User,
};

// Mock GitHub client for testing
pub struct MockGitHubClient {
    username: String,
    /// reviewes keyed by (owner, repo, pr number)
    reviews: HashMap<(String, String, u32), Vec<Review>>,

    /// comments keyed by review_id
    review_comments: HashMap<String, Vec<Comment>>,
}

impl MockGitHubClient {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_string(),
            reviews: Default::default(),
            review_comments: Default::default(),
        }
    }

    pub fn with_reviews<const N: usize>(
        mut self,
        owner: &str,
        repo: &str,
        pr_number: u32,
        reviews: [Review; N],
    ) -> Self {
        self.reviews.insert(
            (owner.to_string(), repo.to_string(), pr_number),
            reviews.to_vec(),
        );
        self
    }

    pub fn with_comments<const N: usize>(
        mut self,
        review_id: &str,
        comments: [Comment; N],
    ) -> Self {
        self.review_comments
            .insert(review_id.to_string(), comments.to_vec());
        self
    }
}

impl GetCurrentUserClient for MockGitHubClient {
    fn get_current_user(&self) -> Result<User> {
        Ok(User {
            login: self.username.clone(),
        })
    }
}

impl ListReviewsClient for MockGitHubClient {
    fn list_reviews(&self, owner: &str, repo: &str, pr_number: u32) -> Result<Vec<Review>> {
        let key = (owner.to_string(), repo.to_string(), pr_number);
        self.reviews
            .get(&key)
            .cloned()
            .ok_or_else(|| anyhow!("Could not find reviews for {key:?}"))
    }
}

impl FetchReviewCommentsClient for MockGitHubClient {
    fn fetch_review_comments(&self, review_id: &str) -> Result<Vec<Comment>> {
        self.review_comments
            .get(review_id)
            .cloned()
            .ok_or_else(|| anyhow!("Could not find comments for review {review_id}"))
    }
}
