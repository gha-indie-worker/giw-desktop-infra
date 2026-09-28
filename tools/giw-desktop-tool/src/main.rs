use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

const GENERATION_AUTHORITY: &str = "7634e94c2051ec473a625ac13a83f329eb66ce2c";
const GENERATION_LIFECYCLE: [&str; 8] = [
    "prepare",
    "validate",
    "compile_build_generation",
    "stage",
    "health_check",
    "atomic_activate",
    "bounded_drain",
    "commit",
];

fn main() -> ExitCode {
    match run() {
        Ok(()) => {
            println!("GIW desktop appliance contract OK");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("GIW desktop appliance contract failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let root = repository_root()?;

    let config_text = fs::read_to_string(root.join(".giw-desktop.toml"))
        .map_err(|error| format!("read .giw-desktop.toml: {error}"))?;
    let config: toml::Value = toml::from_str(&config_text)
        .map_err(|error| format!("parse .giw-desktop.toml: {error}"))?;
    require_toml_str(&config, &["daemon", "listen_addr"], "127.0.0.1:8770")?;
    require_toml_str(
        &config,
        &["execution", "scintilla_url"],
        "http://127.0.0.1:8765",
    )?;
    require_toml_bool(&config, &["execution", "ephemeral_per_job"], true)?;
    require_toml_bool(&config, &["execution", "reuse_workspace"], false)?;

    let schema = read_json(root.join("manifests/scintilla-job-v1.schema.json"))?;
    require_json_bool(&schema, "/properties/ephemeral/const", true)?;
    require_json_bool(&schema, "/properties/reuse_workspace/const", false)?;
    require_json_str(&schema, "/properties/network_mode/const", "restricted")?;

    let ores = read_json(root.join("ores-desktop-appliance.json"))?;
    let daemon_listen = json_str(&ores, "/host/daemon_listen")?;
    if !daemon_listen.starts_with("127.0.0.1:") {
        return Err("host.daemon_listen must be loopback".into());
    }
    require_json_bool(&ores, "/host/requires_public_ip", false)?;
    let common_revision = json_str(&ores, "/common_layer/revision")?;
    require_git_sha("common_layer.revision", common_revision)?;

    let appliance = read_json(root.join("appliance.json"))?;
    require_json_str(&appliance, "/substrate/daemon_url", "http://127.0.0.1:8765")?;
    require_json_bool(&appliance, "/invariants/ephemeral_per_job", true)?;
    require_json_bool(&appliance, "/invariants/workspace_reuse", false)?;
    require_json_bool(&appliance, "/invariants/arbitrary_remote_shell", false)?;
    require_git_sha("substrate.rev", json_str(&appliance, "/substrate/rev")?)?;
    let daemon_revision = appliance
        .pointer("/components/0/rev")
        .and_then(Value::as_str)
        .ok_or_else(|| "components[0].rev is required".to_owned())?;
    require_git_sha("components[0].rev", daemon_revision)?;

    let compose = fs::read_to_string(root.join(".ores-compose.yaml"))
        .map_err(|error| format!("read .ores-compose.yaml: {error}"))?;
    let required_contracts = [
        format!("commit: {daemon_revision}"),
        "GIW_DESKTOP_ADDR: \"127.0.0.1:8770\"".to_owned(),
        "GIW_SCINTILLA_DAEMON_URL: \"http://127.0.0.1:8765\"".to_owned(),
    ];
    for required in &required_contracts {
        if !compose.contains(required) {
            return Err(format!("compose missing required contract: {required}"));
        }
    }
    for forbidden in [
        "0.0.0.0:8770",
        ":latest",
        "cloudflared tunnel run",
        "--allow-non-loopback",
    ] {
        if compose.contains(forbidden) {
            return Err(format!("compose contains forbidden contract: {forbidden}"));
        }
    }

    validate_generation_contract(&root)?;

    return Ok(());
}

fn validate_generation_contract(root: &Path) -> Result<(), String> {
    let contract = read_json(root.join("ores-generation-contract.json"))?;
    require_json_str(
        &contract,
        "/schema",
        "ores.desktop-generation-consumer/v1",
    )?;
    require_json_str(
        &contract,
        "/consumer/repository",
        "gha-indie-worker/giw-desktop-infra",
    )?;
    require_json_str(&contract, "/consumer/role", "desktop_infra")?;
    require_json_str(
        &contract,
        "/authority/repository",
        "ORESoftware/ores-common-desktop-infra",
    )?;
    require_json_str(
        &contract,
        "/authority/revision",
        GENERATION_AUTHORITY,
    )?;
    require_git_sha(
        "authority.revision",
        json_str(&contract, "/authority/revision")?,
    )?;

    let lifecycle = contract
        .pointer("/lifecycle")
        .and_then(Value::as_array)
        .ok_or_else(|| "lifecycle must be an array".to_owned())?;
    if lifecycle.len() != GENERATION_LIFECYCLE.len() {
        return Err("generation lifecycle length mismatch".into());
    }
    for (actual, expected) in lifecycle.iter().zip(GENERATION_LIFECYCLE) {
        if actual.as_str() != Some(expected) {
            return Err(format!(
                "generation lifecycle mismatch: expected {expected:?}, got {actual}"
            ));
        }
    }

    require_json_bool(&contract, "/rollback/required_before_commit", true)?;
    require_json_bool(&contract, "/rollback/retain_previous_generation", true)?;
    require_json_str(
        &contract,
        "/request_semantics/new_requests",
        "active_generation",
    )?;
    require_json_str(
        &contract,
        "/request_semantics/existing_requests",
        "pinned_generation",
    )?;
    require_json_bool(
        &contract,
        "/request_semantics/generation_identity_required",
        true,
    )?;
    require_json_bool(&contract, "/routing/edge_proxy_route_authority", false)?;
    require_json_bool(
        &contract,
        "/middleware/beam_code_reload_requires_drain_or_otp_proof",
        true,
    )?;
    require_json_bool(
        &contract,
        "/verification/shared_conformance_required",
        true,
    )?;
    require_json_bool(&contract, "/verification/product_e2e_required", true)?;

    let role_requirements = contract
        .pointer("/role_requirements")
        .and_then(Value::as_array)
        .ok_or_else(|| "role_requirements must be an array".to_owned())?;
    for required in [
        "build_stage_activate",
        "health_before_activate",
        "stable_ingress_only",
        "scintilla_backed_execution",
        "indiebuild_job_semantics_remain_product_owned",
    ] {
        if !role_requirements.iter().any(|value| value.as_str() == Some(required)) {
            return Err(format!("missing generation role requirement: {required}"));
        }
    }

    return Ok(());
}

fn repository_root() -> Result<PathBuf, String> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    return manifest
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "cannot resolve repository root".into());
}

fn read_json(path: PathBuf) -> Result<Value, String> {
    let text =
        fs::read_to_string(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
    return serde_json::from_str(&text)
        .map_err(|error| format!("parse {}: {error}", path.display()));
}

fn require_git_sha(name: &str, value: &str) -> Result<(), String> {
    let valid = value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if valid {
        return Ok(());
    }

    return Err(format!("{name} must be a 40-character lowercase git SHA"));
}

fn json_str<'a>(value: &'a Value, pointer: &str) -> Result<&'a str, String> {
    return value
        .pointer(pointer)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{pointer} must be a string"));
}

fn require_json_str(value: &Value, pointer: &str, expected: &str) -> Result<(), String> {
    let actual = json_str(value, pointer)?;
    if actual == expected {
        return Ok(());
    }

    return Err(format!("{pointer}={actual:?}, expected {expected:?}"));
}

fn require_json_bool(value: &Value, pointer: &str, expected: bool) -> Result<(), String> {
    let actual = value
        .pointer(pointer)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("{pointer} must be boolean"))?;
    if actual == expected {
        return Ok(());
    }

    return Err(format!("{pointer}={actual}, expected {expected}"));
}

fn toml_at<'a>(value: &'a toml::Value, path: &[&str]) -> Result<&'a toml::Value, String> {
    let mut current = value;
    for part in path {
        current = current
            .get(*part)
            .ok_or_else(|| format!("missing TOML key {}", path.join(".")))?;
    }
    return Ok(current);
}

fn require_toml_str(value: &toml::Value, path: &[&str], expected: &str) -> Result<(), String> {
    let actual = toml_at(value, path)?
        .as_str()
        .ok_or_else(|| format!("{} must be a string", path.join(".")))?;
    if actual == expected {
        return Ok(());
    }

    return Err(format!(
        "{}={actual:?}, expected {expected:?}",
        path.join(".")
    ));
}

fn require_toml_bool(value: &toml::Value, path: &[&str], expected: bool) -> Result<(), String> {
    let actual = toml_at(value, path)?
        .as_bool()
        .ok_or_else(|| format!("{} must be boolean", path.join(".")))?;
    if actual == expected {
        return Ok(());
    }

    return Err(format!("{}={actual}, expected {expected}", path.join(".")));
}
