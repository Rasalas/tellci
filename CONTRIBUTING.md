# Contributing

Thanks for considering a contribution.

`tellci` intentionally stays small. Changes should preserve the core UX:

```bash
tellci pass "Message"
tellci fail "Message"
tellci finish
```

## Local Checks

Run the same checks as CI:

```bash
cargo fmt -- --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
./scripts/ci-tellci.sh
```

## Design Rules

- Keep commands predictable in shell scripts.
- Keep `tellci fail` non-fatal unless `--fatal` is explicitly used.
- Keep the report JUnit-compatible.
- Avoid config files unless they remove clear, repeated pain.
- Prefer focused tests for every behavior change.
