#!/usr/bin/env bash
set -u

TELLCI_BIN="${TELLCI_BIN:-./target/release/tellci}"
TELLCI_FILE="${TELLCI_FILE:-tellci.xml}"
export TELLCI_FILE

"$TELLCI_BIN" reset

check_pass() {
  "$TELLCI_BIN" pass "$1" --class "Project"
}

check_fail() {
  "$TELLCI_BIN" fail "$1" --class "Project" --details "$2"
}

if [[ -f README.md ]]; then
  check_pass "README.md exists"
else
  check_fail "Expected README.md to exist" "The public repository needs a README."
fi

if grep -Eq "GitLab CI|GitHub Actions|JUnit XML|Installation|Usage" README.md; then
  check_pass "README.md documents core usage"
else
  check_fail \
    "Expected README.md to document usage" \
    "README.md should explain CI integration, JUnit XML, installation, and commands."
fi

if [[ -f LICENSE ]]; then
  check_pass "LICENSE exists"
else
  check_fail "Expected LICENSE file to exist" "A public repository should include its license text."
fi

help_output="$("$TELLCI_BIN" --help)"
if grep -Eq "\bpass\b" <<<"$help_output" \
  && grep -Eq "\bfail\b" <<<"$help_output" \
  && grep -Eq "\berror\b" <<<"$help_output" \
  && grep -Eq "\bskip\b" <<<"$help_output" \
  && grep -Eq "\brun\b" <<<"$help_output" \
  && grep -Eq "\bfinish\b" <<<"$help_output" \
  && grep -Eq "\breset\b" <<<"$help_output" \
  && grep -Eq "\bstatus\b" <<<"$help_output"; then
  check_pass "CLI help lists supported commands"
else
  check_fail \
    "Expected CLI help to list supported commands" \
    "The generated help output should mention pass, fail, error, skip, run, finish, reset, and status."
fi

"$TELLCI_BIN" run "tellci version command works" --class "Project" -- "$TELLCI_BIN" --version

"$TELLCI_BIN" finish
