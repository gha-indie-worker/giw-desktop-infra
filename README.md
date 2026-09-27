# giw-desktop-infra

Declarative desired state for running IndieBuild on a developer laptop or workstation.

This repository is intentionally **not** a copy of production `gha-indie-worker-infra`. Desktop mode is
single-host and single-tenant. The production infra repo continues to own hosted/cloud topology, while this
repo defines what one machine should run locally.

## Ownership split

- **`giw-desktop-infra`** — desired state: approved services, argv, environment-variable names, tunnel origin,
  autostart intent, and optional pinned update plan.
- **`giw-desktop-daemon`** — sole machine-local reconciler and lifecycle writer.
- **`giw-desktop-cli`** — operator client of the daemon.
- **`gha-indie-worker-flutter`** — desktop/mobile UI client; desktop builds may control the daemon.
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
             +---- owns -----> local build server / worker processes
             +---- owns -----> cloudflared named tunnel
             +---- owns -----> keep-awake helper
             +---- owns -----> local update argv
```

Clients should never bypass the daemon for normal lifecycle operations. Direct process inspection is diagnostic
only; it must not become a second control loop.

## Files

- `.giw-desktop.yaml` — v1 example desired state consumed by the daemon.
- `schema/giw-desktop.schema.json` — JSON Schema Draft 2020-12 validation contract.

When this contract is promoted to a shared cross-language interface, add the corresponding authored TypeSpec to
`gha-indie-worker-interfaces` and enforce semantic parity with TJSV. Generated schemas are evidence, not a third
authored authority.

## Cloudflare and indiebuild.dev

A developer may expose the local IndieBuild origin through a named Cloudflare Tunnel and an owned
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

## Local build service

The current worker repository still builds the `dd-build-server` binary. The desktop manifest uses that actual
binary name rather than inventing a future executable alias. As the public IndieBuild naming migration proceeds,
change the manifest only when the packaged binary name changes.

The example disables deploy and push by default. Desktop/self-hosted execution should prove local build/test
profiles first; expanding machine authority requires an explicit reviewed config change.

## Secrets

The manifest may list environment **variable names** under `env_passthrough`. Their values come from the local
approved secret boundary and are never serialized through the daemon API. Do not add plaintext `.env` files.
Use the fleet SOPS/age pattern when persistent encrypted configuration is required.

## Reconciliation

Starting the daemon does not imply that every process should run. Services and tunnels have explicit `autostart`
flags. A client may request `POST /v1/reconcile`; the daemon converges only those declared autostart resources.
This keeps GUI restarts, CLI invocations, and Cloudflare reconnects from creating competing supervisors.
