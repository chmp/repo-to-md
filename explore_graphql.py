#!/usr/bin/env python3
"""
Exploration script for GitHub GraphQL API to understand how to fetch PR review comments.

This script tests different GraphQL queries to:
1. Fetch all review comments from a PR
2. List all reviews for a PR
3. Fetch comments from a specific review
4. Compare data structure with REST API

Usage:
    uv run python explore_graphql.py
"""

import json
import subprocess
import sys


def run_gh_graphql(query, variables=None):
    """Execute a GraphQL query using gh CLI."""
    cmd = ["gh", "api", "graphql", "-f", f"query={query}"]

    if variables:
        for key, value in variables.items():
            cmd.extend(["-F", f"{key}={value}"])

    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)
        return json.loads(result.stdout)
    except subprocess.CalledProcessError as e:
        print(f"Error executing gh command: {e.stderr}", file=sys.stderr)
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error parsing JSON response: {e}", file=sys.stderr)
        print(f"Raw output: {result.stdout}", file=sys.stderr)
        sys.exit(1)


def test_basic_pr_query(owner, repo, pr_number):
    """Test basic PR query to verify GraphQL API works."""
    print("=" * 80)
    print("TEST 1: Basic PR Query")
    print("=" * 80)

    query = """
    query($owner: String!, $repo: String!, $prNumber: Int!) {
      repository(owner: $owner, name: $repo) {
        pullRequest(number: $prNumber) {
          id
          number
          title
          state
        }
      }
    }
    """

    result = run_gh_graphql(query, {
        "owner": owner,
        "repo": repo,
        "prNumber": pr_number
    })

    print(json.dumps(result, indent=2))
    return result


def test_list_reviews(owner, repo, pr_number):
    """List all reviews for a PR."""
    print("\n" + "=" * 80)
    print("TEST 2: List All Reviews")
    print("=" * 80)

    query = """
    query($owner: String!, $repo: String!, $prNumber: Int!) {
      repository(owner: $owner, name: $repo) {
        pullRequest(number: $prNumber) {
          reviews(first: 10) {
            nodes {
              id
              state
              author {
                login
              }
              createdAt
              comments(first: 1) {
                totalCount
              }
            }
          }
        }
      }
    }
    """

    result = run_gh_graphql(query, {
        "owner": owner,
        "repo": repo,
        "prNumber": pr_number
    })

    print(json.dumps(result, indent=2))
    return result


def test_fetch_all_review_comments(owner, repo, pr_number):
    """Fetch all review comments from a PR using reviewThreads."""
    print("\n" + "=" * 80)
    print("TEST 3: Fetch All Review Comments via reviewThreads")
    print("=" * 80)

    query = """
    query($owner: String!, $repo: String!, $prNumber: Int!) {
      repository(owner: $owner, name: $repo) {
        pullRequest(number: $prNumber) {
          reviewThreads(first: 20) {
            nodes {
              comments(first: 10) {
                nodes {
                  id
                  body
                  path
                  line
                  startLine
                  diffHunk
                  author {
                    login
                  }
                  pullRequestReview {
                    id
                  }
                }
              }
            }
          }
        }
      }
    }
    """

    result = run_gh_graphql(query, {
        "owner": owner,
        "repo": repo,
        "prNumber": pr_number
    })

    print(json.dumps(result, indent=2))
    return result


def test_fetch_specific_review_comments(review_id):
    """Fetch comments from a specific review using node query."""
    print("\n" + "=" * 80)
    print(f"TEST 4: Fetch Comments from Specific Review: {review_id}")
    print("=" * 80)

    query = """
    query($reviewId: ID!) {
      node(id: $reviewId) {
        ... on PullRequestReview {
          id
          state
          author {
            login
          }
          comments(first: 100) {
            nodes {
              id
              body
              path
              line
              startLine
              diffHunk
              author {
                login
              }
            }
          }
        }
      }
    }
    """

    result = run_gh_graphql(query, {"reviewId": review_id})

    print(json.dumps(result, indent=2))
    return result


def compare_with_rest_api(owner, repo, pr_number):
    """Fetch data via REST API for comparison."""
    print("\n" + "=" * 80)
    print("COMPARISON: REST API Response")
    print("=" * 80)

    api_path = f"/repos/{owner}/{repo}/pulls/{pr_number}/comments"

    try:
        result = subprocess.run(
            ["gh", "api", "-H", "Accept: application/vnd.github+json", api_path],
            capture_output=True,
            text=True,
            check=True
        )
        data = json.loads(result.stdout)

        # Show first comment as sample
        if data:
            print("Sample REST API comment structure:")
            print(json.dumps(data[0], indent=2))
            print(f"\nTotal comments: {len(data)}")
        else:
            print("No comments found")

    except Exception as e:
        print(f"Error fetching REST API data: {e}", file=sys.stderr)


def main():
    # Configuration - update these for your test PR
    OWNER = "chmp"  # Repository owner
    REPO = "review-to-md"  # Repository name
    PR_NUMBER = 1  # PR number to test with

    print("GitHub GraphQL API Exploration")
    print(f"Testing with: {OWNER}/{REPO} PR #{PR_NUMBER}\n")

    # Test 1: Basic PR query
    test_basic_pr_query(OWNER, REPO, PR_NUMBER)

    # Test 2: List reviews
    reviews_result = test_list_reviews(OWNER, REPO, PR_NUMBER)

    # Test 3: Fetch all comments
    all_comments_result = test_fetch_all_review_comments(OWNER, REPO, PR_NUMBER)

    # Test 4: Fetch specific review comments (if reviews exist)
    try:
        reviews = reviews_result["data"]["repository"]["pullRequest"]["reviews"]["nodes"]
        if reviews:
            # Use the first review ID
            review_id = reviews[0]["id"]
            test_fetch_specific_review_comments(review_id)
        else:
            print("\n" + "=" * 80)
            print("SKIP TEST 4: No reviews found")
            print("=" * 80)
    except (KeyError, IndexError) as e:
        print(f"\nCouldn't extract review ID: {e}")

    # Comparison with REST API
    compare_with_rest_api(OWNER, REPO, PR_NUMBER)

    print("\n" + "=" * 80)
    print("Exploration Complete!")
    print("=" * 80)


if __name__ == "__main__":
    main()
