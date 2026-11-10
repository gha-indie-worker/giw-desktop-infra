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

The GIW daemon token defaults to `~/.indiebuild/daemon/token`. The GIW daemon's standalone Scintilla-token default is `~/.scintilla/daemon/token`. Both are local secrets and must never be committed.

The pinned `scintilla-desktop-infra` appliance deliberately keeps its runtime state under its own desktop state directory (`<scintilla checkout>/.desktop/runtime` by default). That means its actual token is normally `<scintilla checkout>/.desktop/runtime/token`, not GIW's standalone fallback path. Do not copy the token. Point GIW at the existing protected file with `GIW_SCINTILLA_TOKEN_FILE`.

## Required local startup order

Scintilla is the execution substrate, so it must be healthy before GIW starts. For the normal sibling-checkout layout:

```sh
SCINTILLA_DIR="$(cd ../scintilla-desktop-infra && pwd)"
SCINTILLA_STATE="${SCINTILLA_DESKTOP_STATE:-$SCINTILLA_DIR/.desktop}"

# Bootstrap/render/start the exact Scintilla revision from appliance.json first.
curl --fail --silent http://127.0.0.1:8765/healthz >/dev/null

test -s "$SCINTILLA_STATE/runtime/token"
export GIW_SCINTILLA_TOKEN_FILE="$SCINTILLA_STATE/runtime/token"

# Only after the Scintilla health and token gates pass should GIW start.
ores-compose check .ores-compose.yaml
ores-compose plan .ores-compose.yaml
ores-compose up .ores-compose.yaml
```

If the Scintilla checkout lives elsewhere, set `SCINTILLA_DESKTOP_STATE` to the state directory created by that appliance before deriving `GIW_SCINTILLA_TOKEN_FILE`.

Starting `giw-desktop-daemon` directly follows the same order: the process reads the Scintilla token during startup and fails closed if the token is missing, unsafe, or unreadable.

## ORES Compose

The product daemon has an exact-revision local lifecycle:

```sh
ores-compose check .ores-compose.yaml
ores-compose plan .ores-compose.yaml
ores-compose up .ores-compose.yaml
```

The Scintilla substrate is installed from the exact revision recorded in `appliance.json`. GIW's compose file deliberately does not start a second Scintilla daemon or expose either daemon publicly. `GIW_SCINTILLA_TOKEN_FILE` is inherited from the caller so the product compose graph can authenticate to the already-running substrate without copying secret material into Git or argv.

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

## Cloudflare admission and live runner release gate

`cloudflared` is an essential outbound connector for **approved product-facing
ingress**, not a substitute for GitHub Actions runner registration. Native
GitHub self-hosted runners receive work over outbound HTTPS to GitHub and do
not need a public listener. The Scintilla substrate (not GIW) must supervise
`cloudflared` and its lifecycle.

The versioned desktop config deliberately has `cloudflare.enabled = false`
and an empty hostname. The executable desktop contract now enforces those
settings, `managed_by = "scintilla"`, and the appliance's explicit
`cloudflare_control_api_exposure = false` and loopback-only invariants. This is
a **fail-closed staging gate**: it does not prevent independently authorized,
Scintilla-managed tunnels for a separately reviewed product origin. Do **not**
set the GIW desktop Cloudflare flag to true just to make a local job run.

Before enabling a GIW-facing Cloudflare route in a future reviewed change,
record live Cloudflare API readback showing the exact named tunnel, ingress
hostname/path/method restrictions, Access application and deny policy,
`cloudflared` version, origin readiness, and a catch-all `404`. A dedicated
signed GitHub webhook surface may use HMAC instead of interactive Access, but
must have independently verified edge path/method blocking. The local GIW
(`127.0.0.1:8770`) and Scintilla (`127.0.0.1:8765`) APIs, tunnel metrics,
local container sockets, and workspace directories must **never** be a
Cloudflare public origin. Keep tunnel credentials outside Git, command-line
arguments, job workspaces, and workflow environment variables; rotate on
suspected exposure.

Run the contract check with `cargo run` and the six Cloudflare negative
regressions with `cargo test` inside `tools/giw-desktop-tool`. Neither static
pass establishes a live tunnel or a real GitHub Actions run. The release
receipt still needs a physical laptop, a GitHub-native `runs-on: self-hosted`
workflow with nonzero executed steps, an isolated one-shot runner, correct
job-run/attempt/report bindings, verified cleanup and a negative Access/HMAC
matrix. See issue #18 for the end-to-end acceptance gate.
