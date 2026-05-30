# Security Policy

## Reporting

Please report security issues privately through GitHub's security advisory
feature when it is available for this repository.

If that is not available, open a minimal public issue that says a private
security report is needed, without including exploit details.

## Scope

`tellci` reads and writes local report files and serializes user-provided text
into XML. Security-sensitive changes include:

- XML escaping and parsing behavior
- filesystem path handling
- command execution features, if added in the future
- release and CI workflows
