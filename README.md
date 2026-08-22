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

Install the latest release:

```bash
curl -fsSL https://github.com/Rasalas/tellci/releases/latest/download/install.sh | sh
```

The installer writes to `/usr/local/bin` when it can. Otherwise it falls back
to `$HOME/.local/bin`.

Install a specific release or directory:

```bash
curl -fsSL https://github.com/Rasalas/tellci/releases/latest/download/install.sh \
  | TELLCI_VERSION=v0.1.1 TELLCI_INSTALL_DIR="$HOME/.local/bin" sh
```

From source:

```bash
cargo install --path .
```

Or build a standalone binary:

```bash
cargo build --release
./target/release/tellci --help
```

Tagged releases publish Linux, macOS, and Windows binaries to GitHub Releases.

To publish a release:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

## Commands

```bash
tellci pass "Message"
tellci fail "Message"
tellci error "Message"
tellci skip "Message"
tellci run "Message" -- command arg
tellci finish
tellci reset
tellci status
```

`tellci run` executes the command after `--` directly, without a shell. A zero
exit status records a pass. A non-zero exit status records a failure. If the
command cannot be started, `tellci` records an error.

Useful options:

```bash
tellci pass "Message" --class "Documentation" --suite "Docs"
tellci fail "Message" --details "Longer explanation"
tellci skip "Message" --details "Why this was skipped"
tellci fail "Blocker" --fatal
tellci error "Tool crashed" --fatal
tellci run "Composer validate works" --fatal -- composer validate
tellci finish --platform github
tellci finish --platform none
tellci --file reports/tellci.xml fail "Message"
TELLCI_FILE=reports/tellci.xml tellci finish
```

## Exit Codes

| Command | Exit code | Meaning |
| --- | ---: | --- |
| `tellci pass "Message"` | `0` | A passing testcase was appended. |
| `tellci fail "Message"` | `0` | A failing testcase was appended, but collection continues. |
| `tellci fail "Message" --fatal` | `1` | A failing testcase was appended and the command fails immediately. |
| `tellci error "Message"` | `0` | An errored testcase was appended, but collection continues. |
| `tellci skip "Message"` | `0` | A skipped testcase was appended. |
| `tellci run "Message" -- command` | `0` | The command was recorded as pass or fail, and collection continues. |
| `tellci run "Message" --fatal -- command` | `1` | The command failed or errored and the command exits immediately. |
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
  before_script:
    - apk add --no-cache curl
    - curl -fsSL https://github.com/Rasalas/tellci/releases/latest/download/install.sh | sh
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

GitHub Actions can show `tellci` output through native job summaries and
annotations.

`tellci finish` auto-detects the current CI provider. On GitHub Actions, it
will:

- append a Markdown summary to `$GITHUB_STEP_SUMMARY`
- emit GitHub error annotations for failed testcases
- keep the regular `finish` exit-code behavior

The summary contains the overall status, report counters, a findings table
when checks fail, and an expandable list of passed checks.

Use `--platform` when you want to override detection:

```bash
tellci finish --platform github
tellci finish --platform gitlab
tellci finish --platform generic
tellci finish --platform none
```

Short aliases are also accepted: `gh`, `ghub`, `gl`, `glab`, `gt`, `off`, and
`detect`.

```yaml
check:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v6
    - name: Install tellci
      run: curl -fsSL https://github.com/Rasalas/tellci/releases/latest/download/install.sh | sudo sh

    - run: |
        test -f README.md \
          && tellci pass "README.md exists" \
          || tellci fail "Expected README.md to exist"

        grep -q "## Installation" README.md \
          && tellci pass "README.md contains Installation section" \
          || tellci fail "Expected README.md to contain ## Installation"

        tellci finish

    - if: always()
      uses: actions/upload-artifact@v7
      with:
        name: tellci-junit
        path: tellci.xml
```

This repository dogfoods `tellci` in its own CI:

```bash
cargo build --release --locked
./scripts/ci-tellci.sh
```

The workflow writes a GitHub job summary, emits annotations when checks fail,
and uploads `tellci.xml` as an artifact.

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
- appending skipped and errored testcases
- running simple commands and recording their result
- producing JUnit-compatible XML
- making the final CI decision with `tellci finish`
- simple metadata such as suite, class, details, and output file

Out of scope for the core MVP:

- replacing a real test framework
- mandatory config files
- a complex rendering phase
- hidden network behavior
