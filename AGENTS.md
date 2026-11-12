
# Rust Project Instructions

## General workflow

- Inspect the existing architecture before making changes.
- Understand the Cargo workspace and dependencies.
- Prefer incremental changes over large rewrites.
- Preserve existing public APIs unless explicitly asked otherwise.
- Avoid modifying unrelated files.

## Rust coding standards

- Write idiomatic, modern Rust.
- Prefer borrowing over unnecessary cloning.
- Use explicit error handling.
- Avoid unwrap() and expect() in production code.
- Prefer clear and maintainable implementations.
- Use appropriate types rather than unnecessary String conversions.
- Avoid unnecessary dependencies.

## Development workflow

Before implementing a feature:

1. Inspect relevant modules.
2. Identify affected interfaces.
3. Explain the intended implementation briefly.
4. Make the changes.

After implementation:

1. Run cargo fmt --all -- --check.
2. Run cargo check --workspace.
3. Run cargo clippy --workspace --all-targets.
4. Run cargo test --workspace.

If a command fails:

- Read the complete error.
- Identify the root cause.
- Make a focused correction.
- Rerun the failed command.
- Do not claim success until validation passes.

## Git safety

- Inspect git status before editing.
- Never discard unrelated changes.
- Never run git reset --hard without permission.
- Never force-push.
- Review the final diff before declaring completion.

## Communication

- Explain architectural decisions.
- Mention assumptions.
- Report failed tests honestly.
- Keep changes focused.

