# Swini Source Code

## Useful Commands

```bash
# Generate the Swini documentation powered by rustdoc and
# open it in the browser
cargo doc --open

# Install just, cargo-nextest, cargo-llvm-cov to run tests.
cargo install just just-lsp cargo-nextest cargo-llvm-cov

# Run all tests:
just test

# Run only unit tests:
just test-unit

# Run only e2e tests:
just test-e2e

# Generate and view test coverage:
just coverage
```
