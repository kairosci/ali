# Delivery jobs

Jobs are small, reviewable units of product work. Every job has an owner, acceptance evidence, data-flow review, and rollback path. Do not mark a job complete from a design document alone.

## Foundation jobs

| ID | Job | Acceptance |
|---|---|---|
| FND-01 | Configuration contract | Every behavior, locale string, prompt, model entry, agent entry, and policy value is parsed from `config/`; missing required keys fail closed. |
| FND-02 | Shared service boundary | CLI and GUI invoke the same typed domain services; adapters do not duplicate policy. |
| FND-03 | Localization completeness | English, Italian, French, and Spanish catalogs have identical required keys and reviewed safety copy. |
| FND-04 | File and state formats | Each primary persisted data structure has a dedicated schema file and versioned migration rules. |

## Privacy and security jobs

| ID | Job | Acceptance |
|---|---|---|
| SEC-01 | Data classification | Every task and payload has a classification and explicit destination decision. Unknown is restricted. |
| SEC-02 | Local document preparation | Supported documents are parsed and cleaned locally; only an explicitly reviewed derived copy can enter a remote request. |
| SEC-03 | Secret boundary | Secret values are fetched by key reference at runtime, redacted from diagnostics, and never serialized. Environment passing is empty by default. |
| SEC-04 | Memory controls | Encrypted local memory has per-agent attribution, retention, export, deletion, backup, and recovery behavior. |
| SEC-05 | Process containment | Agent processes have bounded time/output, explicit cwd, minimal environment, cancellation, and platform isolation policy. |
| SEC-06 | Security review | Threat-led review covers parser attacks, symlinks, local endpoints, browser-origin attacks, vault recovery, and update integrity. |

## Interoperability jobs

| ID | Job | Acceptance |
|---|---|---|
| INT-01 | Adapter SDK contract | Model and agent adapters declare protocol, capabilities, security, bounds, and version compatibility. |
| INT-02 | Local model adapters | Ollama, llama.cpp server, LM Studio, and vLLM compatibility is implemented and verified independently. |
| INT-03 | Remote model adapters | OpenAI-compatible, Anthropic, Google, Azure, and AWS Bedrock adapters use credential references, explicit destination consent, and per-profile policy. |
| INT-04 | Agent adapters | Codex CLI, Claude Code, Gemini CLI, OpenCode, Aider, and user-defined commands have tested invocation profiles and safe I/O contracts. |
| INT-05 | Capability router | Human tasks map to requirements; only compatible, permitted adapters are offered. |
| INT-06 | Failover lifecycle | Supervised agent exit can create an allowlisted <=50-line handoff; explicit approval is required before resuming elsewhere. |

## Product surface jobs

| ID | Job | Acceptance |
|---|---|---|
| UX-01 | CLI workflow | Every supported task is scriptable, help is localized, output is machine-readable by schema, and secrets are not accepted as positional args. |
| UX-02 | Desktop workflow | GUI supports task selection, document review, adapter selection, consent preview, memory controls, and accessibility. |
| UX-03 | Local web workflow | Loopback service passes origin, host, CSRF, CSP, session-capability, and browser storage review. |
| UX-04 | Enterprise controls | Signed policy bundles, identity hooks, retention controls, allowlists, and content-free audit events are documented and validated. |
| UX-05 | Packaging | Signed builds cover Windows, macOS, and Linux desktop; install and update paths are documented. |

## Release jobs

| ID | Job | Acceptance |
|---|---|---|
| REL-01 | Continuous integration | Format, lint, compile, schema validation, and security-focused verification run on Linux, macOS, and Windows. |
| REL-02 | Supply-chain evidence | Lockfile, dependency review, SBOM, provenance, checksums, and signed artifacts are attached to each release. |
| REL-03 | Release automation | Version tags build, validate, sign, publish artifacts, and generate localized release notes with least-privilege tokens. |
| REL-04 | Incident response | Security contact, triage SLA, advisory workflow, supported-version policy, and rollback procedure are published. |

## Definition of done

A job is done only when code, configuration schemas, localized product copy, documentation, compatibility status, and verification evidence agree. The evidence must state platform, tool version, fixtures used, and known limitations. No live confidential user data may be used as a fixture.
