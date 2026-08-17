// This file acts as the single binary entrypoint for all E2E tests.
// Consolidating tests into a single binary is a Rust best practice that
// significantly speeds up compilation time (by only linking dependencies once)
// and visually groups all tests under `swini::e2e` in the test runner.

mod daemon;
