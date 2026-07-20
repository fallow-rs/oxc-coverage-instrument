# Security Policy

## Supported versions

The project is pre-1.0, so fixes land on the newest release rather than on
maintenance branches.

| Version                  | Supported |
| ------------------------ | --------- |
| Latest published release | Yes       |
| Any earlier release      | No        |

Each release is published to [crates.io](https://crates.io/crates/oxc_coverage_instrument)
and to [npm](https://www.npmjs.com/package/oxc-coverage-instrument) from the
same tag, so both registries carry the same fix.

## Reporting a vulnerability

Please do not open a public issue for a security problem. Mail
<bart@fallow.tools> instead.

Useful details to include:

- the affected version and target (native binding, WASM binding, or CLI),
- a minimal input file or command that reproduces the problem,
- what an attacker gains, for example a file write outside the output
  directory during instrumentation, or code execution in the instrumented
  output.

You can expect an acknowledgement within a few days. Once a fix is ready it
ships in a patch release together with a GitHub Security Advisory that credits
the reporter unless anonymity is requested.

## Scope

In scope: the instrumenter, the source-map remapping, the reporters, the CLI,
and the published bindings.

Out of scope: vulnerabilities in a third-party viewer that renders the coverage
output, and issues that require the operator to knowingly run the tool on
hostile input without any resulting privilege gain.
