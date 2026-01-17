# Pull Request Review Comments

Please address the following review comments:

## `src/config.rs` - Lines 12-15

```rust
pub struct Config {
    pub name: String,
    pub value: i32,
// <review user="reviewer1">
// Use a better variable name
// </review>
// <review user="reviewer2">
// Also add validation for this field
// </review>
}
```
