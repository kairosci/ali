# ALI - Agent Local Intelligence

ALI is a local-first workbench for preparing issue briefs, reviewing supported text files locally, encrypted semantic memory, compact agent handoffs, and a native desktop interface. Supervised agent failover and professional document parsing remain roadmap items.

Languages: [English](README.md) · [Italiano](README.it.md) · [Français](README.fr.md) · [Español](README.es.md)

## Security boundary today

- User-interface copy, CLI help, and issue prompts are selected from `config/locales/`, `config/help/`, `config/prompts/`, and `config/model-system/`. Missing translation keys fail; no language fallback is used. Technical diagnostic strings are not fully localized yet.
- No model API key or GitHub token is accepted from command-line arguments or environment variables. Model refinement accepts only a loopback HTTP endpoint. Embeddings accept only a numeric loopback host and never follow redirects.
- Restricted profiles cannot refine a prompt through a model. The app does not launch coding agents yet.
- Restricted profiles also block clipboard export and browser opening from `prepare` and the GUI. These profile controls are user-selected workflow guards, not authentication or an OS security boundary.
- The development profile is restricted. ALI never reads or passes the caller's environment to an agent; agent launches are not implemented.
- Document scanning and semantic review are local-only. `privacy review` embeds the lexically redacted candidate over loopback and writes it only after analysis completes; it does not send documents to a remote service. Configured UTF-8 text formats only are accepted.
- Detection is heuristic. A clean report is not proof that a file contains no sensitive information. Legal, judicial, health, finance, and identity data must remain local.
- Local memory encrypts both text and embeddings with Argon2id-derived XChaCha20-Poly1305 keys. The vault passphrase is not stored. A running same-user process or compromised device can still access plaintext while it is in use.
- Handoff files are capped at the configured limit (never over 50 lines), sanitized with configured patterns, and stored with restrictive permissions. Review them before transferring to another agent.

See [security](docs/SECURITY.md), [threat model](docs/THREAT_MODEL.md), [data lifecycle](docs/DATA_LIFECYCLE.md), and [contributor rules](AGENTS.md).

## Build

Use the pinned Rust 1.100.0 toolchain with mise, or Rust 1.85 or newer locally:

```sh
mise run build
```

The checked-in `Cargo.lock` locks application dependencies. Version tags create cross-platform archives as a draft GitHub release for human review. Artifacts are not signed yet.

Launch the native desktop interface:

```sh
cargo run -- gui
```

List configured model protocols and their delivery status:

```sh
cargo run -- adapters list
```

## Issue brief

```sh
cargo run -- prepare https://github.com/owner/repository/issues/123 --copy --open https://chatgpt.com/
cargo run -- --language it prepare https://github.com/owner/repository/issues/123 --output issue-123.prompt.md
```

Issue retrieval uses the public GitHub API. Private-repository authentication is intentionally not implemented until credentials can be integrated with the platform credential store. Optional model refinement is local-only:

```sh
cargo run -- prepare https://github.com/owner/repository/issues/123 \
  --model-endpoint http://127.0.0.1:11434 --model qwen2.5-coder
```

ALI does not install, authenticate, or launch the model server. Outbound model calls to non-loopback destinations are rejected.

## Local memory

Start the configured local Ollama service and make the configured embedding model available yourself. ALI never downloads a model automatically.

```sh
cargo run -- memory init
cat notes.txt | cargo run -- memory add --agent-id codex-cli
printf '%s' 'previous decision about access control' | cargo run -- memory search --agent-id codex-cli
cargo run -- memory list
cargo run -- memory clear
```

The passphrase is entered through a hidden terminal prompt. Memory text, an optional configured agent tag, and vectors are encrypted together. Tags are manual; ALI does not harvest agent activity. Agent-filtered search includes shared notes and notes tagged for that agent. The embedding request goes only to the numeric loopback endpoint in `config/runtime.toml`. Override storage with `memory --data-dir /private/path`; ALI rejects storage directories with broad Unix permissions or symlinks.

The native GUI also supports vault initialization, adding notes, listing notes, and searching by an optional configured agent. It clears passphrase and query fields when an operation starts and zeroizes sensitive GUI fields when the window closes. Vault clearing and storage-path overrides remain CLI operations.

## Local document review

```sh
cargo run -- --language en privacy scan --input confidential.txt --output confidential.review.txt
cargo run -- --language en privacy analyze --input confidential.txt
cargo run -- --language en privacy review --input confidential.txt --output confidential.candidate.txt
```

The configured scanners cover selected email, token, private-key, tax-code, US SSN, and payment-card patterns. `privacy analyze` compares bounded original-text segments with configured multilingual sensitivity descriptions using the loopback embedding service. `privacy review` first creates an in-memory pattern-redacted candidate, semantically analyzes that candidate locally, then writes it only after analysis succeeds. Neither command transfers a document to a remote service. Review the entire candidate and every semantic cue locally. Similarity is not a calibrated probability, proof of detection, legal review, privilege protection, anonymization, or permission to send. Names, case facts, indirect identifiers, images, handwriting, and unknown formats can be missed. PDF and office documents are rejected until safe, bounded parsers are implemented.

## Hibernation handoff

Provide the documented schema as JSON on stdin; ALI rejects unknown fields and multiline values:

```sh
cat handoff.json | cargo run -- handoff create
```

Required fields: `session_id`, `failed_agent`, `failure` (`crash`, `timeout`, `non_zero_exit`, `token_limit`, `cancelled`, or `unknown`), `verified_progress` (up to eight single-line strings), and `next_action`. The resulting local Markdown file is scanned, line-counted, and written without overwriting an existing file. Automatic agent failover is not yet implemented. `ali agents list` checks configured executable paths only; it does not verify versions, authentication, or safety, and does not run agents.

## Configuration and localization

| Path | Parsed content |
|---|---|
| `config/runtime.toml` | Languages, profiles, outbound policy, local embedding service, storage identity, document types, memory KDF policy, and bounds. |
| `config/providers.toml` | GitHub API and compatible local completion path. |
| `config/sensitivity.toml` | Local sensitive-pattern detectors. |
| `config/privacy-review.toml` | Multilingual local semantic probes, chunk/batch/time bounds, and similarity threshold. |
| `config/help/{en,it,fr,es}.md` | Localized CLI command guide shown by `--help`. |
| `config/locales/{en,it,fr,es}.toml` | Required CLI messages and security/consent wording. |
| `config/prompts/{en,it,fr,es}.md` | Localized issue brief templates. |
| `config/model-system/{en,it,fr,es}.md` | Localized model refinement instructions. |
| `config/handoffs/{en,it,fr,es}.md` | Localized compact handoff templates. |
| `config/agents.toml` | Explicit agent CLI catalogue. Current adapters are inspection-only. |
| `config/adapters.toml` | Local and remote protocol catalog with explicit implementation status. |

The `[memory_policy]` section bounds individual entries, encrypted vault size, list results, and search results for both CLI and GUI.

Configuration is loaded from `config/` relative to the current working directory. `--config-dir` or `ALI_CONFIG_DIR` can select another complete configuration directory. `ALI_AGENT_URL` supplies the optional browser destination for `prepare`. The config is parsed and all supported locale keys and templates are validated before work begins.

## Current and planned scope

Current CLI commands are `prepare`, `memory`, `privacy scan`, `privacy analyze`, `privacy review`, `handoff create`, `agents list`, and `gui`. Agent execution/failover, authenticated app profiles, PDF/Office parsing, and platform installers remain explicit milestones. Track them in [the roadmap](docs/ROADMAP.md). See [architecture](docs/ARCHITECTURE.md) and the four localized [user guides](README.it.md), [French](README.fr.md), and [Spanish](README.es.md).

## Adapter catalog

List configured protocols with `ali adapters list`. The catalog is parsed from `config/adapters.toml`; entries marked `planned` describe roadmap candidates and are not executable integrations. The current OpenAI-compatible chat client and Ollama embeddings are loopback-only. See [interoperability](docs/INTEROPERABILITY.md), [delivery jobs](docs/JOBS.md), and the [roadmap](docs/ROADMAP.md).
