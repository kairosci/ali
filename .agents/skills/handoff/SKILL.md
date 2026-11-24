# Agent handoff skill

Use this skill when starting, supervising, resuming, or replacing an agent.

- Keep one durable local session record with task ID, issue/document references, agent identity, state, and timestamps. Do not store credentials or raw restricted documents in the handoff.
- Before launching an agent, inspect its configured name, executable identity, version, capability grant, network policy, working directory, and environment allowlist.
- Do not claim to know every agent installed. Report only agents detected from explicit configured executable paths and current version probes.
- Use a defined adapter protocol. Never assume all agent CLIs interpret stdin, flags, output, or exit status the same way.
- On timeout, non-zero exit, crash, or malformed output, freeze that attempt, capture bounded diagnostics locally, and generate an allowlisted hibernation note.
- The hibernation file must contain at most 50 lines, include verified facts and one next action, contain no secrets or raw user documents, and be reviewed locally before transfer.
- Continue with the next user-approved configured agent only when its capability and data grants cover the same task. Never broaden access during failover.
- If no authorized compatible agent remains, preserve the session and ask the user to select/install/configure one. Never install software automatically.
- Record each transition and the reason. Do not discard the failed agent's worktree or overwrite uncommitted changes.
