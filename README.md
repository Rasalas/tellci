# tellci

[![CI](https://github.com/Rasalas/tellci/actions/workflows/ci.yml/badge.svg)](https://github.com/Rasalas/tellci/actions/workflows/ci.yml)

`tellci` is a tiny CLI that writes simple CI feedback messages to a JUnit XML
report, so GitLab can show them in merge request test reports.

It is not a test framework. It is a small JUnit report writer for CI scripts.

## Why

CI scripts often contain small checks that do not belong in a full test
framework:

- Does `README.md` exist?
- Does the README contain an installation section?
- Did a one-off shell command produce the expected output?
- Is a generated file present?

Plain shell output is easy to miss in CI logs. JUnit XML gives CI systems a
structured report format. GitLab can display that report directly in merge
requests.

## Quick Start

```bash
tellci pass "Composer install works"
tellci fail "Expected README.md to contain ## Installation"
tellci fail "Expected LICENSE file to exist"
tellci finish
```

By default, `tellci` writes:

```text
tellci.xml
```

`tellci fail` records a failure but exits with `0`, so a CI script can collect
multiple findings. `tellci finish` exits with `1` if the report contains any
failure or error.

The generated report looks like this:

```xml
<testsuite name="tellci" tests="3" failures="2" errors="0" skipped="0">
  <testcase classname="tellci" name="Composer install works"/>
  <testcase classname="tellci" name="Expected README.md to contain ## Installation">
    <failure message="Expected README.md to contain ## Installation">Expected README.md to contain ## Installation</failure>
  </testcase>
  <testcase classname="tellci" name="Expected LICENSE file to exist">
    <failure message="Expected LICENSE file to exist">Expected LICENSE file to exist</failure>
  </testcase>
</testsuite>
```

## Installation

From source:

```bash
cargo install --path .
```

Or build a standalone binary:

```bash
cargo build --release
./target/release/tellci --help
```

When tagged releases are used, the release workflow builds Linux, macOS, and
Windows binaries as GitHub Actions artifacts.

## Commands

```bash
tellci pass "Message"
tellci fail "Message"
tellci finish
tellci reset
tellci status
```

Useful options:

```bash
tellci pass "Message" --class "Documentation" --suite "Docs"
tellci fail "Message" --details "Longer explanation"
tellci fail "Blocker" --fatal
tellci --file reports/tellci.xml fail "Message"
TELLCI_FILE=reports/tellci.xml tellci finish
```

## Exit Codes

| Command | Exit code | Meaning |
| --- | ---: | --- |
| `tellci pass "Message"` | `0` | A passing testcase was appended. |
| `tellci fail "Message"` | `0` | A failing testcase was appended, but collection continues. |
| `tellci fail "Message" --fatal` | `1` | A failing testcase was appended and the command fails immediately. |
| `tellci finish` | `0` | The report contains no failures or errors. |
| `tellci finish` | `1` | The report contains at least one failure or error. |
| Any command with an I/O or XML error | `2` | The report could not be read, parsed, or written. |

## File Selection

The default report path is `tellci.xml`.

Use `--file`:

```bash
tellci --file reports/tellci.xml fail "Expected README.md to exist"
tellci --file reports/tellci.xml finish
```

Or use `TELLCI_FILE`:

```bash
TELLCI_FILE=reports/tellci.xml tellci fail "Expected README.md to exist"
TELLCI_FILE=reports/tellci.xml tellci finish
```

## GitLab CI

```yaml
check:
  stage: test
  image: alpine:latest
  script:
    - test -f README.md
      && tellci pass "README.md exists"
      || tellci fail "Expected README.md to exist"

    - grep -q "## Installation" README.md
      && tellci pass "README.md contains Installation section"
      || tellci fail "Expected README.md to contain ## Installation"

    - test -f LICENSE
      && tellci pass "LICENSE exists"
      || tellci fail "Expected LICENSE file to exist"

    - tellci finish
  artifacts:
    when: always
    reports:
      junit: tellci.xml
```

## GitHub Actions

GitHub Actions does not display JUnit reports in pull requests the same way
GitLab does, but `tellci` is still useful there:

- collect several script-level findings before failing the job
- upload `tellci.xml` as an artifact
- use the same checks locally, in GitHub Actions, and in GitLab CI

This repository dogfoods `tellci` in its own CI:

```bash
cargo build --release --locked
./scripts/ci-tellci.sh
```

The workflow then uploads `tellci.xml` as an artifact.

## Development

Run the full local check set:

```bash
cargo fmt -- --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release
./scripts/ci-tellci.sh
```

## Project Scope

`tellci` should stay boring and script-friendly.

In scope:

- appending passing and failing testcases
- producing JUnit-compatible XML
- making the final CI decision with `tellci finish`
- simple metadata such as suite, class, details, and output file

Out of scope for the core MVP:

- replacing a real test framework
- mandatory config files
- a complex rendering phase
- hidden network behavior

## License

MIT
