# Security and privacy model

## Scope and current implementation

ALI is a local-first workbench, not an audited security product. This version has no ALI account system, app-level sign-in, multi-user isolation, or OS keychain integration. Access to the running app and decrypted memory relies on the user's operating-system account. A process running as that same user can inspect data in use. Do not treat the profile selector as authentication or authorization.

The CLI and native GUI can fetch public GitHub issue content when explicitly invoked. Optional model refinement and semantic memory embedding use only configured numeric loopback HTTP endpoints, with proxy use disabled and redirects rejected. The GUI can open a user-provided HTTP(S) destination after generating a brief; that action does not submit the brief. Any later manual paste into a third-party site is outside ALI's local boundary.

This source has not undergone an independent security audit. Local processing can still be exposed by a compromised device, malware, backups, swap, screenshots, clipboard managers, the logged-in local account, vulnerable dependencies, or external applications.

## Data classes

| Class | Examples | Default handling |
|---|---|---|
| Public | Public issue URL and public issue text | Local fetch only when requested; no model call by default. |
| Internal | Source code, unpublished tasks, internal notes | Local-only; outbound transfer requires a payload preview and explicit approval. |
| Restricted | Credentials, personal data, legal/medical/financial records, privileged communications | No outbound transfer. Local encrypted storage only where enabled. |
| Derived restricted | Embeddings, summaries, redaction maps, crash notes | Inherit the source's classification and retention. |

## Identity and authentication

There is no ALI sign-in or user account in this version. The encrypted memory vault has a separate passphrase, prompted without terminal echo and never stored by ALI. Provider credentials and agent authentication are not managed by ALI. No agent process is launched by the current agent catalogue. Restricted profiles block model refinement, clipboard export, and browser opening in the current brief workflow; these are user-selected application guards, not authentication. The development profile is restricted. ALI does not read the caller's environment or pass it to agents. Do not put credentials in issue text, prompts, memory, handoffs, or command-line arguments.

## Outbound controls

- Public issue retrieval is an explicit HTTPS request to the configured GitHub API. No private-repository credential flow exists.
- Model refinement is restricted to a configured numeric loopback HTTP endpoint; restricted profiles cannot use it. The endpoint's process and host remain part of the trusted computing base.
- The GUI can open an HTTP(S) URL, but does not send its prompt to that destination. Manual copy/paste is a user-controlled transfer outside ALI's enforcement boundary.
- Redaction is a best-effort local transformation followed by local review. It does not make a document anonymous or establish that legal privilege is preserved.
- Embeddings are sensitive derived data and remain local. They are useful for retrieval and review; they are not a reliable detector or sanitizer by themselves.
- Redirects to unapproved destinations, non-HTTPS remote endpoints, and non-loopback local model endpoints are rejected by the policy layer.

## Persistence and deletion

Persistent memory is opt-in and encrypted at rest with Argon2id-derived keys and XChaCha20-Poly1305. The vault passphrase is not stored. Unix data directories and files use restrictive permissions and symlink checks. Memory is currently one vault per selected data directory, not a per-profile account store. `memory clear` deletes that vault file; this cannot guarantee physical erasure from SSDs, snapshots, backups, or filesystem journals. Memory export and expiry are not implemented.

## Incident handling

On a suspected disclosure, stop outbound operations, preserve only necessary local diagnostics, rotate affected credentials at their issuer, and document the facts and scope. Do not send the incident payload to another model for analysis. See [the threat model](THREAT_MODEL.md) and [data lifecycle](DATA_LIFECYCLE.md).
