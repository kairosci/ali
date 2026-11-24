# ALI product brief

## Problem

Starting coding-agent work from an issue repeatedly requires collecting the issue context, explaining the repository workflow, and restating expectations around evidence and verification. Manual prompt drafting is repetitive and can omit useful context.

## Product direction

Provide a cross-platform Rust workbench with CLI and native GUI. It prepares source-grounded issue briefs, scans supported text documents locally, stores optional encrypted semantic memory, and creates bounded handoff notes. Prompt content, locale catalogues, provider settings, detector patterns, and agent catalogue are parsed from `config/`.

## Primary user and outcome

A person supplies an issue URL or text document and receives a locally reviewable brief or redacted candidate. They can encrypt local memories and create a compact session handoff. The native GUI exposes the supported issue and document flows in four locales.

## MVP acceptance criteria

1. Accept HTTPS GitHub issue and pull request URLs and reject unsupported or malformed URLs with an actionable error.
2. Fetch public GitHub issue metadata and bounded comments without credentials.
3. Render a localized prompt from parsed Markdown templates.
4. Support stdout, file output, clipboard copy, and explicitly opening an agent website without automatic paste.
5. Permit local-only model refinement and loopback embedding requests.
6. Encrypt user-selected memory and embeddings at rest with a passphrase-derived key.
7. Scan configured UTF-8 text types locally into a separate redacted candidate and show residual-risk warnings.
8. Provide an explicit local embedding review for text segments using configured multilingual sensitivity descriptions; cues never authorize transfer.
9. Create a validated, sensitive-pattern-scanned handoff no longer than 50 lines.
10. Provide CLI and native GUI surfaces using the same configuration and services.
11. Fail visibly on missing configuration, locale, local model, or secure-storage conditions.

## Non-goals for the first release

- Automatically controlling a browser or pasting into an agent UI.
- Cloning or modifying the issue's repository from the prompt generator.
- Creating branches, commits, issues, or pull requests.
- Launching coding agents or automatic failover (planned milestones).
- Installing models or agents.
- Claiming that a downstream coding agent completed work.

## Product decisions

| Decision | Rationale |
|---|---|
| Local-first CLI and GUI | Supports terminal workflows and a native desktop surface without a network control plane. |
| User reviews and pastes | Prevents a site launch from silently submitting a prompt. |
| Markdown templates in configuration | Lets maintainers tune the actual prompt without Rust source edits. |
| Local-only OpenAI-compatible model protocol | Keeps document content on the device; non-loopback endpoints are rejected in this release. |
| GitHub first | Defines a testable initial provider; provider expansion is a roadmap item. |
