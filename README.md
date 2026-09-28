# GIW Desktop Infra

Single-host desired-state authority for running IndieBuild / GHA Indie Worker jobs on developer- or end-user-owned desktops.

GIW is **not** a second generic desktop scheduler. The installed `scintilla-run/scintilla-desktop-infra` appliance owns long-lived host/container lifecycle. GIW owns GitHub-compatible workflow/job intent, runner registration semantics, job fencing, cancellation, logs, artifacts, and execution policy.

```text
IndieBuild hosted control plane
          |
          | outbound authenticated control/lease path
          v
 giw-desktop-daemon :8770
          |
          | typed indiebuild.scintilla/v1 ephemeral-job projection
          v
 Scintilla desktop daemon :8765
          |
          +---- ephemeral Linux/container runner
          +---- optional native macOS runner
          +---- optional native Windows runner
          +---- optional substrate-managed cloudflared
```

## Ownership boundary

- **Scintilla owns machine/process/container lifecycle.**
- **`giw-desktop-daemon` is the IndieBuild-facing machine-local API and Scintilla adapter.**
- **Every untrusted CI job gets an ephemeral execution identity and workspace.**
- **GIW clients submit typed job intent, never executable argv or shell commands.**
- **CLI / Rust desktop app / Flutter clients never launch runners, containers, or tunnels directly.**

## Local configuration

`.giw-desktop.toml` is the product policy authority. The expected local endpoints are:

- GIW daemon: `127.0.0.1:8770`
- Scintilla desktop daemon: `127.0.0.1:8765`

The GIW daemon token defaults to `~/.indiebuild/daemon/token`; the Scintilla token defaults to `~/.scintilla/daemon/token`. Both are local secrets and must never be committed.

## ORES Compose

The product daemon has an exact-revision local lifecycle:

```sh
ores-compose check .ores-compose.yaml
ores-compose plan .ores-compose.yaml
ores-compose up .ores-compose.yaml
```

The Scintilla substrate is installed from the exact revision recorded in `appliance.json`. GIW's compose file deliberately does not start a second Scintilla daemon or expose either daemon publicly.

## Cloudflare

A public IP is not required. Normal runner/control traffic is outbound. When a product-facing local service needs public ingress, Scintilla owns the optional Cloudflare Tunnel process and publishes only an explicitly allow-listed product origin. The GIW and Scintilla loopback control APIs are never tunnel origins.

## CI isolation

Untrusted workflow code is the highest-trust-risk workload in this family. The required default is:

- one ephemeral runtime/workspace per job;
- no cross-job workspace reuse;
- non-root container execution where the job is Linux/container-compatible;
- no privileged mode, host networking, host PID/IPC, hostPath, or ambient service-account token;
- bounded CPU, memory, disk, process count, and wall-clock timeout;
- private/link-local/metadata network destinations blocked unless a reviewed job capability explicitly requires otherwise;
- secrets scoped to one job and destroyed with the execution;
- native macOS/Windows execution is explicit opt-in because it has a broader host trust boundary.

The job projection schema is `manifests/scintilla-job-v1.schema.json`.

## Shared common layer

This repo pins `ORESoftware/ores-common-desktop-infra` in `ores-desktop-appliance.json`. Generic lifecycle/security behavior belongs there; GIW-specific workflow semantics stay here and in the GIW daemon/runtime repositories.
