# ALI - command guide

ALI keeps document review, memory, and model analysis on this device by default. This guide is loaded from the selected configuration directory.

## Global options

- `--config-dir PATH`: choose a complete configuration directory. Default: `config`; override: `ALI_CONFIG_DIR`.
- `--language en|it|fr|es`: select the interface and prompt language. Default: `product.default_locale` in `runtime.toml`.
- No command, `--help`, `-h`, or `help`: show this configured guide.

## Commands

- `ali prepare URL [--output PATH] [--copy] [--open URL] [--model-endpoint LOOPBACK_URL --model NAME] [--profile PROFILE] [--agent-id ID]`: retrieve a public GitHub issue over HTTPS and render a local brief. Model refinement is optional, local-only, and unavailable to restricted profiles. Opening a website never pastes the brief.
- `ali memory init|add|search|list|clear [--data-dir PATH]`: manage the encrypted local memory vault. `add` and `search` read text from standard input; add `--agent-id ID` to tag a note or search shared notes plus notes tagged for that configured agent. Ollama must already be running at the configured loopback endpoint for embedding operations.
- `ali privacy scan --input PATH --output PATH`: locally scan a configured UTF-8 text file and create a separate redacted candidate.
- `ali privacy analyze --input PATH`: compare bounded local text segments with configured multilingual sensitivity descriptions using the loopback embedding service. Similarity cues are not a release decision.
- `ali privacy review --input PATH --output PATH`: apply configured pattern redaction locally, semantically review the redacted candidate through loopback embeddings, then write the candidate only if local review completes. This command never transfers the document to a remote service.
- `ali handoff create [--output PATH]`: read the validated JSON handoff schema from standard input and create a locally scanned note of at most 50 lines.
- `ali agents list`: inspect only agent command paths declared in `config/agents.toml`; this does not launch agents.
- `ali web`: open the loopback-only local browser interface. It prepares public GitHub issue prompts only; it does not accept document uploads.
- `ali gui`: open the native desktop interface.

## Data boundary

ALI has no account system and does not launch agents in this version. Public issue retrieval is an explicit GitHub network request. Document scans, semantic review, and memory embeddings use local processing; embedding endpoints must be numeric loopback addresses. The semantic review is an incomplete aid, not a guarantee of detection, anonymity, legal privilege, or permission to disclose. Keep restricted data local and review all output yourself.

### adapters list

Show configured model protocols and their implementation status. Entries marked planned are not usable yet.
