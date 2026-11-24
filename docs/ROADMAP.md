# ALI milestones and delivery roadmap

## North star

ALI is a local-first workbench that helps people move between models and agents while preserving task context, user control, and data-handling policy. People express the outcome they need. ALI prepares the context, identifies compatible tools, explains the destination and data flow, and keeps a compact local record that can travel with the task when approved.

The product targets desktop operating systems and web browsers, with native GUI, local web UI, and CLI surfaces. It does not target mobile. English, Italian, French, and Spanish are the initial product locales. Product copy, prompt templates, provider and agent catalogs, policy values, and operational help are parsed from `config/`. Rust source defines schema keys and behavior, not user-facing content or model catalogs.

## Delivery principles

- Build a trustworthy local core before adding remote reach.
- Add protocols as reviewed adapters. Do not promise every model forever; keep the catalog extensible and publish tested compatibility per release.
- Generate agent-specific prompts from task intent, agent capabilities, repository instructions, locale, and approved context. Do not expose hidden reasoning or invent facts.
- Keep one feature or primary persisted data structure per file, with explicit schemas and migrations.
- Make a user approve external destination and exact payload. Restricted material remains local.
- No environment dumps. Secret values never enter prompts, logs, configuration, memory, handoffs, or model requests.
- Embeddings help find sensitive context. They do not prove that a payload is safe.

## Milestones

### M0: Safe local foundation, current

Delivered: Rust CLI and native desktop shell, required configuration parsing, four locale catalogs and templates, public GitHub issue brief flow, encrypted opt-in memory, configured agent discovery, local lexical scan, local embedding review, and <=50-line manual hibernation files. Linux formatting, Clippy, and optimized build have passed locally. Cross-platform CI is configured but has not run in this workspace.

Exit evidence: all configuration parses without fallback, outbound actions are explicit, local data paths are bounded and private, and limitations are visible in product docs.

### M1: Shared task model and configuration contract

Deliver one typed task representation shared by CLI and GUI. Add dedicated schema files for tasks, data classifications, model capabilities, agent capabilities, memory entries, audit events, and handoffs. Add migrations, schema validation, localization parity, and generated configuration reference docs. Remove remaining hardcoded user-facing copy from source.

Exit evidence: CLI and GUI produce the same task plan from the same inputs, config validation reports exact keys and files, and locale catalogs pass parity checks.

### M2: Secure local document pipeline

Add locally parsed document formats, extraction limits, archive bomb defense, active content stripping, local classification, configurable detectors, redaction review UI, and safe export. Use a configurable local embedding model to locate risk passages. Support text first, then PDF and Office formats only after parser sandboxing and threat review.

Exit evidence: original bytes stay local; any outbound derivative is shown, scanned, and approved; unknown formats fail closed; redaction claims are bounded.

### M3: Adapter framework and local model breadth

Specify a versioned adapter contract and capability handshake. Implement local server adapters in separate increments: Ollama, llama.cpp server, LM Studio, and vLLM. Add model discovery only as an explicit local action. Keep endpoints and model IDs in `config/`; no baked model list in Rust.

Exit evidence: each adapter has protocol fixtures, local loopback policy, bounded response parsing, cancellation, and a published compatibility record.

### M4: Remote model providers and consent router

Add remote provider adapters as individually reviewed jobs. Initial candidates are OpenAI-compatible APIs, Anthropic, Google, Azure-hosted models, and AWS Bedrock. Use OS credential storage and secret references. Show provider, model, region, data classes, retention terms, and exact outbound payload before sending. Provider support is feature-gated by explicit user or enterprise policy.

Exit evidence: no provider request occurs before consent, secrets are not exposed to the model or logs, and restricted profiles block remote transfer regardless of provider.

### M5: Agent supervision and recovery

Implement version and capability probes, dedicated adapters, explicit working directory, empty-by-default environment, bounded IO, cancellation, and operating-system sandbox integration. Generate per-agent prompts from configured templates and capabilities. Detect exits only under an explicitly supervised run. Write a redacted <=50-line hibernation file and require approval before resuming on another agent.

Exit evidence: failure injection demonstrates no environment leakage, handoff validation and line cap are enforced, and each adapter publishes its supported mode and known risks.

### M6: Human task layer and usable surfaces

Add outcome-based workflows for understand, plan, implement, review, research, and handoff. The capability router recommends tools that satisfy task, privacy, and policy constraints. Ship equivalent CLI and native GUI workflows. Add a local web UI behind a loopback-only capability endpoint. Design for keyboard access, screen readers, localization, and clear consent.

Exit evidence: the user can see why an adapter was selected, change it, inspect context, and stop a transfer from every surface.

### M7: Enterprise and operational readiness

Add signed managed policy bundles, organization identity integration, allowlisted destinations, retention controls, content-free audit events, administrative compatibility catalog, and offline deployment. Add packaging, signed builds, SBOM, provenance, vulnerability response, support policy, backups, and upgrade migration guides.

Exit evidence: independent security and accessibility reviews completed, platform-specific storage and sandbox behavior reviewed, and no unresolved critical/high findings.

## Current status

M0 is implemented as a limited foundation. M1 through M7 are planned. See [jobs](JOBS.md) for independently reviewable work and acceptance evidence. See [interoperability](INTEROPERABILITY.md) for adapter support policy.

## Release gates

| Release | Scope | Gate |
|---|---|---|
| 0.1 | Local CLI and native GUI foundation | Current source compiles locally; limitations documented; no production security claim. |
| 0.2 | Shared task and config schemas | Four-locale parity, migration behavior, and CLI/GUI parity verified. |
| 0.3 | Local document pipeline | Parser threat review, local review UI, and retention controls complete. |
| 0.4 | Local model adapters | Adapter-specific compatibility and bounded protocol verification complete. |
| 0.5 | Remote providers and agent supervision | Consent, secret management, process containment, and failure handoff reviewed. |
| 0.8 | Local web and enterprise preview | Browser security, identity boundaries, policy distribution, and audit minimization reviewed. |
| 1.0 | General availability | Signed cross-platform builds, SBOM, accessibility and locale review, incident response, and independent security review complete. |
