# giw-desktop-infra

Declarative desired state for running IndieBuild orchestration on a developer laptop or workstation.

This repository is intentionally **not** a copy of production `gha-indie-worker-infra`, and GIW desktop is
**not** a seventh FaaS runtime. Desktop mode is single-host and single-tenant. The production infra repo owns
hosted/cloud topology; this repo defines GIW-local orchestration helpers plus the typed adapter configuration
required to reach Scintilla for worker/lambda execution.

## Ownership split

- **`giw-desktop-infra`** — desired state for GIW-local orchestration helpers, tunnel origin, autostart intent,
  update policy, and the GIW -> Scintilla adapter endpoint/config references.
- **`giw-desktop-daemon`** — sole GIW machine-local reconciler and lifecycle writer for those helper processes.
- **Scintilla desktop runtime** — sole local FaaS worker/lambda lifecycle authority: launch, invoke, cancel,
  status, logs/evidence, deployment revision, and teardown.
- **`giw-desktop-cli`** — operator client of the GIW daemon.
- **`gha-indie-worker-flutter`** — desktop/mobile UI client; desktop builds may control the GIW daemon.
- **`gha-indie-worker-desktop-app.rs`** — native Rust desktop UI client.
- **`gha-indie-worker-infra`** — production/hosted infrastructure; never implicitly mutated by desktop reconcile.

Canonical flow:

```text
CLI / Flutter / Rust desktop UI
             |
             v
     giw-desktop-daemon
             |
             +---- reads ----> .giw-desktop.yaml
             |
             +---- owns -----> GIW local orchestration helpers
             +---- owns -----> cloudflared named tunnel
             +---- owns -----> keep-awake helper
             +---- owns -----> local GIW update argv
             |
             +---- adapts ---> Scintilla desktop daemon
                                   |
                                   +--> worker/lambda launch + invoke
                                   +--> cancellation + status + logs
                                   +--> deployment revision + teardown
```

Clients must never bypass the GIW daemon for GIW lifecycle operations, and GIW must never bypass Scintilla by
launching a worker/lambda through the generic helper-service mechanism. Direct process inspection is diagnostic
only; it must not become a second control loop.

The GIW -> Scintilla boundary must preserve a stable correlation/job identity end to end. If Scintilla is
unavailable or its protocol is incompatible, GIW fails closed and reports the dependency failure; it does not
silently fall back to a GIW-owned worker process.

## Files

- `.giw-desktop.yaml` — v1 example desired state consumed by the daemon. Its `services` entries are GIW-local
  orchestration helpers only, never FaaS workers.
- `schema/giw-desktop.schema.json` — JSON Schema Draft 2020-12 validation contract.

When this contract is promoted to a shared cross-language interface, add the corresponding authored TypeSpec to
`gha-indie-worker-interfaces` and enforce semantic parity with TJSV. Generated schemas are evidence, not a third
authored authority.

## Cloudflare and indiebuild.dev

A developer may expose the local IndieBuild orchestration origin through a named Cloudflare Tunnel and an owned
`*.indiebuild.dev` hostname. The manifest stores only the tunnel **name**, public hostname, and loopback origin.
Cloudflare credentials remain in `cloudflared`'s normal local credential store; API tokens and tunnel credentials
must never be committed here.

The example uses:

```yaml
tunnel:
  name: indiebuild-desktop
  hostname: device.indiebuild.dev
  service_url: http://127.0.0.1:8080
  autostart: false
```

Operators should replace `device.indiebuild.dev` with the device hostname assigned by the IndieBuild control
plane. DNS/tunnel enrollment is a separate authenticated step from starting the local tunnel process.

## Local build orchestration service

The current worker repository still builds the `dd-build-server` binary. In GIW desktop mode this binary is an
**orchestration/build-control helper**, not the authority that directly creates FaaS workers. Any actual worker
execution requested by that service must use the versioned GIW -> Scintilla adapter. The desktop manifest uses
the packaged binary name rather than inventing a future executable alias; change it only when packaging changes.

The example disables deploy and push by default. Desktop/self-hosted execution should prove local build/test
profiles first; expanding machine authority requires an explicit reviewed config change.

## Secrets

The manifest may list environment **variable names** under `env_passthrough`. Their values come from the local
approved secret boundary and are never serialized through the daemon API. Do not add plaintext `.env` files.
Use the fleet SOPS/age pattern when persistent encrypted configuration is required.

Scintilla authentication must be referenced through a protected local token/credential-file boundary. Do not
copy token contents into `.giw-desktop.yaml`, argv, logs, artifacts, or UI state.

## Reconciliation

Starting the GIW daemon does not imply that every GIW helper process should run. Services and tunnels have
explicit `autostart` flags. A client may request `POST /v1/reconcile`; the daemon converges only those declared
GIW-local resources. Worker/lambda desired state is reconciled through the Scintilla adapter, not through the
GIW generic process map. This keeps GUI restarts, CLI invocations, Scintilla restarts, and Cloudflare reconnects
from creating competing supervisors.
