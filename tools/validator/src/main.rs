#![forbid(unsafe_code)]

use std::{collections::HashSet, env, fs, path::Path};

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let manifest_path = args
        .next()
        .unwrap_or_else(|| ".giw-desktop.yaml".to_string());
    let schema_path = args
        .next()
        .unwrap_or_else(|| "schema/giw-desktop.schema.json".to_string());

    if args.next().is_some() {
        bail!("usage: giw-desktop-infra-validator [manifest.yaml] [schema.json]");
    }

    validate_schema_document(Path::new(&schema_path))?;
    let manifest = load_yaml_as_json(Path::new(&manifest_path))?;
    validate_manifest(&manifest)?;

    println!("GIW desktop infra validation passed: {manifest_path}");
    return Ok(());
}

fn validate_schema_document(path: &Path) -> Result<()> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read JSON Schema at {}", path.display()))?;
    let schema: Value = serde_json::from_str(&raw)
        .with_context(|| format!("invalid JSON Schema JSON at {}", path.display()))?;
    let object = schema
        .as_object()
        .context("desktop JSON Schema root must be an object")?;

    if object.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        bail!("desktop schema must declare JSON Schema Draft 2020-12");
    }

    return Ok(());
}

fn load_yaml_as_json(path: &Path) -> Result<Value> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read desktop manifest at {}", path.display()))?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&raw)
        .with_context(|| format!("invalid YAML at {}", path.display()))?;
    return serde_json::to_value(yaml).context("failed to normalize manifest to JSON value");
}

fn validate_manifest(value: &Value) -> Result<()> {
    let root = value
        .as_object()
        .context("desktop manifest root must be an object")?;

    reject_unknown_keys(root, &["version", "services", "tunnel", "update"], "manifest")?;

    if root.get("version").and_then(Value::as_u64) != Some(1) {
        bail!("desktop manifest version must be exactly 1");
    }

    let services = root
        .get("services")
        .and_then(Value::as_array)
        .context("desktop manifest services must be an array")?;
    let mut names = HashSet::new();

    for (index, service) in services.iter().enumerate() {
        validate_service(service, index, &mut names)?;
    }

    if let Some(tunnel) = root.get("tunnel") {
        validate_tunnel(tunnel)?;
    }

    if let Some(update) = root.get("update") {
        validate_update(update)?;
    }

    return Ok(());
}

fn validate_service(value: &Value, index: usize, names: &mut HashSet<String>) -> Result<()> {
    let service = value
        .as_object()
        .with_context(|| format!("services[{index}] must be an object"))?;
    reject_unknown_keys(
        service,
        &[
            "name",
            "command",
            "args",
            "working_dir",
            "env_passthrough",
            "env",
            "autostart",
        ],
        &format!("services[{index}]"),
    )?;

    let name = required_nonempty_string(service, "name", &format!("services[{index}]"))?;
    if !is_service_name(name) {
        bail!("services[{index}].name contains unsupported characters: {name:?}");
    }
    if !names.insert(name.to_string()) {
        bail!("duplicate desktop service name {name:?}");
    }

    let _ = required_nonempty_string(service, "command", &format!("services[{index}]"))?;

    if let Some(args) = service.get("args") {
        validate_string_array(args, &format!("services[{index}].args"), false)?;
    }

    if let Some(working_dir) = service.get("working_dir") {
        if working_dir.as_str().is_none_or(str::is_empty) {
            bail!("services[{index}].working_dir must be a non-empty string");
        }
    }

    if let Some(passthrough) = service.get("env_passthrough") {
        validate_string_array(
            passthrough,
            &format!("services[{index}].env_passthrough"),
            true,
        )?;
        for key in passthrough.as_array().expect("validated as array") {
            let key = key.as_str().expect("validated as string");
            if !is_env_name(key) {
                bail!("services[{index}].env_passthrough has invalid env name {key:?}");
            }
        }
    }

    if let Some(env) = service.get("env") {
        let env = env
            .as_object()
            .with_context(|| format!("services[{index}].env must be an object"))?;
        for (key, literal_value) in env {
            if !is_env_name(key) {
                bail!("services[{index}].env has invalid env name {key:?}");
            }
            if literal_value.as_str().is_none() {
                bail!("services[{index}].env.{key} must be a string");
            }
            if looks_secret_bearing(key) {
                bail!(
                    "services[{index}].env.{key} looks secret-bearing; list the variable name under env_passthrough or use the encrypted secret boundary instead of a literal value"
                );
            }
        }
    }

    if let Some(autostart) = service.get("autostart")
        && !autostart.is_boolean()
    {
        bail!("services[{index}].autostart must be boolean");
    }

    return Ok(());
}

fn validate_tunnel(value: &Value) -> Result<()> {
    let tunnel = value.as_object().context("tunnel must be an object")?;
    reject_unknown_keys(
        tunnel,
        &["name", "hostname", "service_url", "autostart"],
        "tunnel",
    )?;

    let _ = required_nonempty_string(tunnel, "name", "tunnel")?;
    let service_url = required_nonempty_string(tunnel, "service_url", "tunnel")?;
    if !is_loopback_http_url(service_url) {
        bail!("tunnel.service_url must be loopback HTTP; got {service_url:?}");
    }

    if let Some(hostname) = tunnel.get("hostname")
        && hostname.as_str().is_none_or(str::is_empty)
    {
        bail!("tunnel.hostname must be a non-empty string when present");
    }

    if let Some(autostart) = tunnel.get("autostart")
        && !autostart.is_boolean()
    {
        bail!("tunnel.autostart must be boolean");
    }

    return Ok(());
}

fn validate_update(value: &Value) -> Result<()> {
    let update = value.as_object().context("update must be an object")?;
    reject_unknown_keys(update, &["command", "args"], "update")?;
    let _ = required_nonempty_string(update, "command", "update")?;
    if let Some(args) = update.get("args") {
        validate_string_array(args, "update.args", false)?;
    }
    return Ok(());
}

fn reject_unknown_keys(object: &Map<String, Value>, allowed: &[&str], context: &str) -> Result<()> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            bail!("{context} contains unknown key {key:?}");
        }
    }
    return Ok(());
}

fn required_nonempty_string<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<&'a str> {
    let value = object
        .get(key)
        .and_then(Value::as_str)
        .with_context(|| format!("{context}.{key} must be a string"))?;
    if value.is_empty() {
        bail!("{context}.{key} may not be empty");
    }
    return Ok(value);
}

fn validate_string_array(value: &Value, context: &str, unique: bool) -> Result<()> {
    let values = value
        .as_array()
        .with_context(|| format!("{context} must be an array"))?;
    let mut seen = HashSet::new();

    for entry in values {
        let entry = entry
            .as_str()
            .with_context(|| format!("{context} entries must be strings"))?;
        if unique && !seen.insert(entry) {
            bail!("{context} contains duplicate entry {entry:?}");
        }
    }

    return Ok(());
}

fn is_service_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    return chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '_' | '-'));
}

fn is_env_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first != '_' && !first.is_ascii_uppercase() {
        return false;
    }
    return chars.all(|ch| ch == '_' || ch.is_ascii_uppercase() || ch.is_ascii_digit());
}

fn looks_secret_bearing(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    return [
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PRIVATE",
        "CREDENTIAL",
        "API_KEY",
        "DATABASE_URL",
        "GH_PAT",
    ]
    .iter()
    .any(|marker| upper.contains(marker));
}

fn is_loopback_http_url(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("http://") else {
        return false;
    };
    let authority = rest.split('/').next().unwrap_or_default();
    return authority == "localhost"
        || authority.starts_with("localhost:")
        || authority == "127.0.0.1"
        || authority.starts_with("127.0.0.1:")
        || authority == "[::1]"
        || authority.starts_with("[::1]:");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_literal_names_are_rejected() {
        assert!(looks_secret_bearing("SERVER_AUTH_SECRET"));
        assert!(looks_secret_bearing("DATABASE_URL"));
        assert!(!looks_secret_bearing("BUILD_SERVER_DEPLOY_ENABLED"));
    }

    #[test]
    fn tunnel_origin_is_loopback_only() {
        assert!(is_loopback_http_url("http://127.0.0.1:8080"));
        assert!(is_loopback_http_url("http://localhost:8080/path"));
        assert!(!is_loopback_http_url("https://127.0.0.1:8080"));
        assert!(!is_loopback_http_url("http://indiebuild.dev:8080"));
    }

    #[test]
    fn environment_names_are_strict() {
        assert!(is_env_name("GIW_DESKTOP_URL"));
        assert!(is_env_name("_GIW_TEST"));
        assert!(!is_env_name("giw_desktop_url"));
    }
}
