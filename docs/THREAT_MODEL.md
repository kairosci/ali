# Threat model

## Assets

User documents and source code; credentials and identity data; local memory and embeddings; active prompts and agent handoffs; user consent and authorization; integrity of repository changes.

## Adversaries and accidents

- Malicious instructions in issues, PDFs, office documents, source comments, or retrieved memory.
- A compromised or over-permissioned agent/model/provider.
- A local process, shared account, clipboard manager, backup, or diagnostic collector.
- A user selecting the wrong profile, agent, file, destination, or redacted output.
- Malicious model output attempting path traversal, shell execution, or unauthorized data transfer.
- Bugs in parsers, template rendering, platform credential storage, dependencies, and update mechanisms.

## Trust boundaries

1. User input to parser and file adapters.
2. Local app process to local model and embedding processes.
3. Local app to configured agent subprocess.
4. Local app to any network endpoint.
5. Encrypted local store to OS account and backup systems.
6. Hibernation handoff from failed agent to replacement agent.

## Principal risks and controls

| Risk | Controls | Residual risk |
|---|---|---|
| Prompt injection exfiltrates a document | Current: no agent subprocess is launched; model refinement is loopback-only and restricted profiles block it. | Local model output can mislead a human; opening an external site and manual paste are outside ALI's enforcement boundary. |
| Redactor misses sensitive text | Local deterministic detectors, local review, restricted data stays local | No detector can prove a mixed legal document contains no sensitive or privileged content. |
| Semantic similarity misses sensitive content or flags ordinary text | Configured local probes, bounded chunks, explicit warning, local review | Embedding similarity is model- and language-dependent and never establishes safe disclosure. |
| Handoff contains a secret | Fixed schema, 50 physical lines, bounded source facts, final local scan and review | A secret can be encoded or paraphrased beyond detector capability. |
| Agent inherits credentials | Current: agent catalogue is inspection-only; ALI launches no agent. | Future process adapters must define an explicit environment allowlist, worktree boundary, and capability policy before launch is implemented. |
| Memory theft | Current: opt-in encrypted vault, passphrase-derived key, restrictive Unix permissions, symlink checks. | No OS keychain integration; same-user malware can access memory while unlocked. |
| Local web UI CSRF or DNS rebinding | Ephemeral numeric loopback bind, one-use bootstrap capability, exact Host checks, exact Origin checks for mutations, SameSite Strict HttpOnly session cookie, no CORS, restrictive CSP, no-store responses, bounded requests. | Same-user local malware can inspect browser state; this is not user authentication. Browser vendors, extensions, and local processes remain in the trusted computing base. |
| Data left after deletion | Delete records, vector indexes, temp files, and keys; disclose backup limits | Flash translation layers and external backups prevent guaranteed physical erasure. |

## Security invariants

- No network side effect occurs implicitly from parsing an issue or opening a document.
- No restricted item crosses a network boundary.
- No agent receives the inherited process environment.
- No model output is executed as a command or treated as authorization.
- No fallback changes a failed secure operation into plaintext, broader permissions, or a different destination.
- Failure to classify, scan, decrypt, authenticate, or authorize means stop and preserve local data.

## Verification work

The project roadmap requires adversarial fixtures, local traffic observation, process-environment tests, storage-permission checks across operating systems, and an independent security review before a production security claim.
