# GHA Indie Worker — giw-desktop-infra

Declarative desired state for single-machine IndieBuild desktop/self-hosted execution.

- This repo declares topology; it does not supervise processes itself.
- `giw-desktop-daemon` is the sole machine-local reconciler and lifecycle writer.
- CLI/Flutter/Rust desktop clients talk to the daemon, not directly to `cloudflared`, build workers, or local services.
- Production `gha-indie-worker-infra` remains authoritative for hosted/cloud deployments.
- Never commit Cloudflare credentials, API tokens, decrypted SOPS data, `.env` files, or bearer tokens.
- Prefer schema-validated config. JSON Schema Draft 2020-12 and authored TypeSpec remain peer authorities when a cross-runtime contract is promoted into `gha-indie-worker-interfaces`.
- Rust-first tooling; no Python checks or codegen.
- Resolve conflicts semantically; do not rebase, stash, reset, or force-push shared history.

For fleet-wide rules, read `ORESoftware/my-ai` `AGENTS.md` and `SHARED.md`.
