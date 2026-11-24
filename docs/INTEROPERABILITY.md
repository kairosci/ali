# Model and agent interoperability

## Product promise

ALI is an agent workbench and router. It gives people one place to prepare context, choose a compatible model or agent, apply data-handling rules, and keep a portable session record. Providers and agents remain replaceable. ALI does not claim that one protocol can control every tool or that every model is safe for every task.

## Support policy

There is no permanent, complete list of every model in the world. Model names and provider APIs change continuously. ALI therefore separates protocol adapters from configured catalogs:

1. Rust code implements reviewed protocol adapters and validation rules.
2. `config/` declares provider endpoints, model identifiers, capabilities, locale labels, and policy requirements.
3. Users can add compatible local or remote entries without changing Rust code.
4. Discovery is optional, explicit, bounded, and never uploads prompts or documents.
5. Unknown capabilities are treated as unavailable. ALI never guesses an adapter or silently downgrades a security policy.

The current implementation supports the OpenAI-compatible chat shape only at numeric loopback endpoints, and Ollama embeddings at numeric loopback endpoints. The agent catalog discovers configured executable paths but does not yet invoke them. Other protocols and automatic failover are roadmap work, not current features.

## Adapter contract

Each model adapter declares a protocol key, endpoint policy, authentication reference mechanism, request and response bounds, timeout, streaming behavior, tool-call behavior, structured-output support, and data classes it may receive. Secret values are provided through an OS credential store or an explicitly selected process environment key at runtime. Secret values are never stored in TOML, Markdown, prompts, logs, memory, handoffs, or command history. The key name may appear in configuration; the secret value may not.

Each agent adapter declares a stable id, executable resolution policy, supported invocation modes, input/output channel, working-directory policy, environment allowlist, filesystem/network permissions, cancellation behavior, and handoff compatibility. Agent input is generated from the selected task, locale, agent capability profile, repository rules, and user-approved context. The current issue-brief flow appends one validated localized Markdown working agreement selected by the configured agent profile. Prompt templates and all user-facing values are parsed from `config/`.

Agent execution is disabled until the process supervisor enforces an empty-by-default environment, explicit working directory, bounded arguments and output, timeout/cancellation, and OS isolation where available. Local execution does not make a tool harmless: an agent can read files allowed by the operating system.

## Human task layer

Users choose an outcome such as understand, plan, implement, review, research, or prepare a handoff. A task policy maps that outcome to required capabilities, data classification, evidence requirements, and approval points. ALI then selects only adapters satisfying those constraints. The task layer may ask a human to choose when multiple compatible agents remain. It must not hide the destination or data being transferred.

## Failover and hibernation

Failover is opt-in per task. Before switching agents, ALI records only allowlisted state: task objective, completed work, verified facts, open questions, constraints, and next action. The hibernation document is generated locally, scanned, and limited to 50 physical lines. Credentials, environment values, unreviewed source, and hidden chain-of-thought are excluded. A replacement agent receives the handoff only after the user approves the destination and payload. The current build supports manual creation, not automatic death detection or resume.

## Enterprise deployment

Enterprise policy is additive and deny-by-default. An administrator may distribute signed configuration bundles, allowlisted protocols and destinations, retention rules, approved local models, identity integration, audit-event schemas, and managed update channels. Local privacy controls remain visible to the user. Audit events contain action identifiers and policy outcomes, not prompt bodies, document content, credentials, or environment values. Enterprise policy cannot silently enable remote transfer of restricted data.

## Desktop and web surfaces

The CLI, native desktop GUI, and local browser UI call the same prompt preparation services and configuration. The browser UI binds only to IPv4 loopback on an ephemeral port, uses a one-time local capability, validates Host and Origin, applies a restrictive CSP, and avoids remote assets and CORS. It currently accepts public GitHub issue URLs only; it does not upload documents, call models, or submit prompts. A hosted web service is a separate deployment with explicit tenancy, identity, key custody, retention, and threat model. Mobile is outside the product target.

## Compatibility matrix

The compatibility matrix is generated from validated configuration and adapter metadata. Entries distinguish `implemented`, `experimental`, and `planned`. A model identifier in a catalog does not itself imply compatibility. Release notes must name tested adapter versions and supported capabilities. See [roadmap](ROADMAP.md) for delivery gates.
