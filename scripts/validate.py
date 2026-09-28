#!/usr/bin/env python3
import json
import pathlib
import re
import sys
import tomllib

root = pathlib.Path(__file__).resolve().parents[1]

with (root / ".giw-desktop.toml").open("rb") as f:
    cfg = tomllib.load(f)

assert cfg["daemon"]["listen_addr"] == "127.0.0.1:8770"
assert cfg["execution"]["scintilla_url"] == "http://127.0.0.1:8765"
assert cfg["execution"]["ephemeral_per_job"] is True
assert cfg["execution"]["reuse_workspace"] is False

schema = json.loads((root / "manifests/scintilla-job-v1.schema.json").read_text())
assert schema["properties"]["ephemeral"]["const"] is True
assert schema["properties"]["reuse_workspace"]["const"] is False
assert schema["properties"]["network_mode"]["const"] == "restricted"

ores = json.loads((root / "ores-desktop-appliance.json").read_text())
assert ores["host"]["daemon_listen"].startswith("127.0.0.1:")
assert ores["host"]["requires_public_ip"] is False
assert ores["common_layer"]["revision"] == "1de34a491673cff2ff7fedb6ba36f8b6a10ae5a1"
assert re.fullmatch(r"[0-9a-f]{40}", ores["common_layer"]["revision"])

appliance = json.loads((root / "appliance.json").read_text())
assert appliance["substrate"]["daemon_url"] == "http://127.0.0.1:8765"
assert appliance["invariants"]["ephemeral_per_job"] is True
assert appliance["invariants"]["workspace_reuse"] is False
assert appliance["invariants"]["arbitrary_remote_shell"] is False

compose = (root / ".ores-compose.yaml").read_text()
for required in [
    "commit: 26fe0e8e4078e69c8ae6f045d7d0a91ea9bf77dd",
    'GIW_DESKTOP_ADDR: "127.0.0.1:8770"',
    'GIW_SCINTILLA_DAEMON_URL: "http://127.0.0.1:8765"',
]:
    assert required in compose, required

for forbidden in ["0.0.0.0:8770", "latest", "cloudflared tunnel run"]:
    assert forbidden not in compose, forbidden

print("GIW desktop appliance contract OK")
