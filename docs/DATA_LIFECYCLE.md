# Data lifecycle

| Stage | Data | Location | Network rule | Retention |
|---|---|---|---|---|
| Import | Issue URL, selected documents | Process memory; temporary encrypted staging where required | No network unless GitHub fetch is explicitly requested | Removed at session close unless saved |
| Parse and classify | Extracted text and classification | Local process | Semantic review sends bounded chunks only to the configured loopback embedding service; no remote model transfer is implemented for documents | Ephemeral by default |
| Embed and retrieve | Chunks and vectors | Encrypted local memory/index | Embedding endpoint must be configured local and loopback | User-configured; source classification inherited |
| Redact and review | Original, candidate redacted copy, local mapping | Local process and encrypted staging | No outbound transfer during scan; restricted documents remain local | Candidate discarded unless user saves it |
| Prompt assembly | Issue, selected context, workflow | Local process | No network model call by default | Not persisted unless requested |
| Agent run | Not supported in this version | No agent subprocess is launched | No agent traffic from ALI | Not applicable |
| Handoff | Minimal state and next action | Local file created only by explicit CLI command | Local scan; user must review before manually passing it on | User deletes the created file |
| Memory | User-selected durable facts, with optional configured-agent attribution | Shared passphrase-encrypted local vault | Embedding request goes only to configured numeric loopback endpoint, after vault unlock | CLI and GUI can initialize, add, list, and search; CLI clears the vault. Entry, vault, list, and result bounds come from `config/runtime.toml`. Expiry/export are not implemented; agent tags are manual |

Users must treat system backups, swap, clipboard history, and manually selected agent services as separate retention systems. The GUI's website action only opens a URL; any subsequent paste is outside ALI's data flow and controls.
