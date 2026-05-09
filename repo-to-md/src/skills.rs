// Embedded skill files for installation
pub const SKILLS: &[(&str, &str)] = &[
    (
        "issue-to-md",
        include_str!("../../skills/issue-to-md/SKILL.md"),
    ),
    (
        "local-review",
        include_str!("../../skills/local-review/SKILL.md"),
    ),
    (
        "review-to-md",
        include_str!("../../skills/review-to-md/SKILL.md"),
    ),
];
