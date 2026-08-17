# List all available commands
default:
  @just --list

# Run all tests (unit + e2e)
test:
  cargo nextest run

# Run only unit tests
test-unit:
  cargo nextest run -E 'kind(lib) | kind(bin)'

# Run only e2e tests
test-e2e:
  cargo nextest run -E 'kind(test)'

# Generate an HTML test coverage report and open it
coverage:
  cargo llvm-cov nextest --open
