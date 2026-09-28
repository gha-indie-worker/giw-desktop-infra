#![forbid(unsafe_code)]

use std::{collections::HashSet, env, fs, path::Path};

use anyhow::{Context, Result, bail};
use serde_json::Value;

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;

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

    let schema = load_and_validate_schema(Path::new(&schema_path))?;
    let manifest = load_yaml_as_json(Path::new(&manifest_path))?;
    validate_against_schema(&schema, &manifest)?;
    validate_semantics(&manifest)?;

    println!("GIW desktop infra validation passed: {manifest_path}");
    return Ok(());
}

fn load_and_validate_schema(path: &Path) -> Result<Value> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read JSON Schema at {}", path.display()))?;
    let schema: Value = serde_json::from_str(&raw)
        .with_context(|| format!("invalid JSON Schema JSON at {}", path.display()))?;
    if schema.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        bail!("desktop schema must declare JSON Schema Draft 2020-12");
    }
    jsonschema::meta::validate(&schema)
        .map_err(|error| anyhow::anyhow!("desktop authored JSON Schema is invalid: {error}"))?;
    return Ok(schema);
}

fn load_yaml_as_json(path: &Path) -> Result<Value> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("failed to stat desktop manifest at {}", path.display()))?;
    if metadata.len() > MAX_MANIFEST_BYTES {
        bail!("desktop manifest exceeds 1 MiB: {}", path.display());
    }
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read desktop manifest at {}", path.display()))?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&raw)
        .with_context(|| format!("invalid YAML at {}", path.display()))?;
    return serde_json::to_value(yaml).context("failed to normalize manifest to JSON value");
}

fn validate_against_schema(schema: &Value, manifest: &Value) -> Result<()> {
    let validator = jsonschema::validator_for(schema)
        .map_err(|error| anyhow::anyhow!("failed to compile authored JSON Schema: {error}"))?;
    let errors = validator
        .iter_errors(manifest)
        .take(20)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        bail!(
            "desktop manifest failed authored JSON Schema validation:\n- {}",
            errors.join("\n- ")
        );
    }
    return Ok(());
}

fn validate_semantics(value: &Value) -> Result<()> {
    let root = value
        .as_object()
        .context("desktop manifest root must be an object after schema validation")?;
    let services = root
        .get("services")
        .and_then(Value::as_array)
        .context("desktop manifest services must be an array after schema validation")?;
    let mut names = HashSet::new();

    for (index, service) in services.iter().enumerate() {
        let service = service
            .as_object()
            .with_context(|| format!("services[{index}] must be an object"))?;
        let name = service
            .get("name")
            .and_then(Value::as_str)
            .with_context(|| format!("services[{index}].name missing after schema validation"))?;
        if !names.insert(name) {
            bail!("duplicate desktop service name {name:?}");
        }
        if let Some(env) = service.get("env").and_then(Value::as_object) {
            for key in env.keys() {
                if looks_secret_bearing(key) {
                    bail!(
                        "services[{index}].env.{key} looks secret-bearing; list the variable name under env_passthrough or use the encrypted secret boundary instead of a literal value"
                    );
                }
            }
        }
    }

    if let Some(tunnel) = root.get("tunnel").and_then(Value::as_object) {
        let service_url = tunnel
            .get("service_url")
            .and_then(Value::as_str)
            .context("tunnel.service_url missing after schema validation")?;
        validate_literal_loopback_origin(service_url)?;
    }

    return Ok(());
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

fn validate_literal_loopback_origin(value: &str) -> Result<()> {
    let without_scheme = value
        .strip_prefix("http://")
        .context("tunnel.service_url must use http://")?;
    let authority = without_scheme.strip_suffix('/').unwrap_or(without_scheme);
    if authority.contains(['/', '?', '#', '@']) {
        bail!(
            "tunnel.service_url must be a literal loopback origin without credentials, path, query, or fragment"
        );
    }

    let port = if let Some(port) = authority.strip_prefix("127.0.0.1:") {
        port
    } else if let Some(port) = authority.strip_prefix("[::1]:") {
        port
    } else {
        bail!("tunnel.service_url host must be literal 127.0.0.1 or ::1");
    };
    let port: u16 = port
        .parse()
        .with_context(|| "tunnel.service_url must contain an explicit valid TCP port")?;
    if port == 0 {
        bail!("tunnel.service_url port 0 is not allowed");
    }
    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn authored_schema() -> Value {
        return serde_json::from_str(include_str!("../../../schema/giw-desktop.schema.json"))
            .expect("authored schema JSON");
    }

    fn valid_manifest() -> Value {
        return json!({
            "version": 1,
            "services": [{
                "name": "build-server",
                "command": "dd-build-server",
                "args": [],
                "env_passthrough": ["SERVER_AUTH_SECRET"],
                "env": {"BUILD_SERVER_DEPLOY_ENABLED": "false"},
                "autostart": false
            }],
            "tunnel": {
                "name": "indiebuild-desktop",
                "hostname": "device.indiebuild.dev",
                "service_url": "http://127.0.0.1:8080",
                "autostart": false
            }
        });
    }

    #[test]
    fn authored_schema_is_valid_draft_2020_12() {
        let schema = authored_schema();
        assert_eq!(
            schema.get("$schema").and_then(Value::as_str),
            Some("https://json-schema.org/draft/2020-12/schema")
        );
        assert!(jsonschema::meta::validate(&schema).is_ok());
    }

    #[test]
    fn checked_in_shape_passes_schema_and_semantics() {
        let schema = authored_schema();
        let manifest = valid_manifest();
        assert!(validate_against_schema(&schema, &manifest).is_ok());
        assert!(validate_semantics(&manifest).is_ok());
    }

    #[test]
    fn secret_literal_is_rejected_by_schema_and_semantics() {
        let schema = authored_schema();
        let mut manifest = valid_manifest();
        manifest["services"][0]["env"] = json!({"API_TOKEN": "synthetic-canary"});
        assert!(validate_against_schema(&schema, &manifest).is_err());
        assert!(validate_semantics(&manifest).is_err());
    }

    #[test]
    fn ambiguous_loopback_urls_are_rejected() {
        let schema = authored_schema();
        for candidate in [
            "http://localhost:8080",
            "http://user@127.0.0.1:8080",
            "http://127.0.0.1:8080/path",
            "http://127.0.0.1:8080?x=1",
            "https://127.0.0.1:8080",
        ] {
            let mut manifest = valid_manifest();
            manifest["tunnel"]["service_url"] = Value::String(candidate.to_string());
            assert!(
                validate_against_schema(&schema, &manifest).is_err(),
                "{candidate}"
            );
            assert!(validate_semantics(&manifest).is_err(), "{candidate}");
        }
    }

    #[test]
    fn semantic_port_check_rejects_out_of_range_port() {
        let schema = authored_schema();
        let mut manifest = valid_manifest();
        manifest["tunnel"]["service_url"] = Value::String("http://127.0.0.1:99999".to_string());
        assert!(validate_against_schema(&schema, &manifest).is_ok());
        assert!(validate_semantics(&manifest).is_err());
    }
}
