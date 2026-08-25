# Security policy

ZiFile treats every archive and its metadata as untrusted input. Please do not
open a public issue containing an unpatched vulnerability, working exploit,
malicious archive, password, credential, or private customer data.

## Reporting a vulnerability

GitHub private vulnerability reporting is not currently enabled for this
repository. Until it is enabled, report vulnerabilities privately to
`ax2@zicode.com` with the subject `ZiFile security report`. Include the affected
version or commit, impact, reproduction steps, and a minimal non-sensitive test
case when practical. If private vulnerability reporting is enabled later, the
repository Security page will become the preferred channel.

Receipt and remediation times depend on severity and maintainer availability;
ZiFile does not currently promise a fixed response SLA. The maintainer will
coordinate disclosure and credit with the reporter when contact details are
provided. Please allow a reasonable remediation period before public disclosure.

## Supported versions

Before the first stable release, only the current default branch receives
security fixes. After 1.0, supported release lines will be listed here before an
older line stops receiving fixes.

## Scope

Reports about archive parsing, path traversal, links and reparse points,
resource exhaustion, password exposure, privilege-boundary mistakes, updater or
package integrity, and unsafe shell integration are in scope. Reports that only
describe a missing feature, require the user to intentionally run unrelated
untrusted software, or concern unsupported third-party builds are normally out
of scope.

The detailed threat model and implemented controls are documented in the
[security model](docs/src/content/docs/architecture/security.md).
