use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use audio_feature_lab_config::FeatureName;
use quick_xml::Reader;
use quick_xml::events::Event;
use serde_json::{Value as JsonValue, json};

const MPEG7_ENV_JAR: &str = "MPEG7_AUDIOENC_JAR";
const MPEG7_ENV_JAVA: &str = "JAVA_BIN";

pub(crate) fn backend_version() -> Result<String, BackendError> {
    let runtime = resolve_runtime()?;
    ensure_java_available(&runtime.java_bin)?;

    Ok(format!(
        "mpeg7audioenc external-java ({})",
        runtime.jar_path.display()
    ))
}

pub(crate) fn analyze_file(path: &Path, config_json: &str) -> Result<String, BackendError> {
    let runtime = resolve_runtime()?;
    ensure_java_available(&runtime.java_bin)?;

    let request = parse_request(config_json)?;
    let config_xml = render_config_xml(&request);
    let config_path = write_temp_config(&config_xml)?;

    let output = Command::new(&runtime.java_bin)
        .arg("-jar")
        .arg(&runtime.jar_path)
        .arg(path)
        .arg(&config_path)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| BackendError::Spawn {
            command: format_command(
                &runtime.java_bin,
                [
                    runtime.jar_path.as_os_str(),
                    path.as_os_str(),
                    config_path.as_os_str(),
                ],
            ),
            error,
        })?;

    let _ = fs::remove_file(&config_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if stderr.is_empty() {
            format!("process exited with status {}", output.status)
        } else {
            stderr
        };
        return Err(BackendError::CommandFailed(detail));
    }

    let xml = String::from_utf8(output.stdout).map_err(BackendError::InvalidUtf8)?;
    let payload = build_payload_from_xml(&request, &xml)?;
    serde_json::to_string(&payload).map_err(BackendError::SerializePayload)
}

#[derive(Debug)]
pub enum BackendError {
    Unavailable(String),
    InvalidRequest(String),
    Io {
        path: PathBuf,
        error: std::io::Error,
    },
    Spawn {
        command: String,
        error: std::io::Error,
    },
    CommandFailed(String),
    InvalidUtf8(std::string::FromUtf8Error),
    Xml(quick_xml::Error),
    SerializePayload(serde_json::Error),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(message) => write!(f, "mpeg7 backend is unavailable: {message}"),
            Self::InvalidRequest(message) => {
                write!(f, "mpeg7 backend rejected the request: {message}")
            }
            Self::Io { path, error } => write!(f, "failed to access {}: {error}", path.display()),
            Self::Spawn { command, error } => {
                write!(f, "failed to spawn `{command}`: {error}")
            }
            Self::CommandFailed(message) => write!(f, "mpeg7 command failed: {message}"),
            Self::InvalidUtf8(error) => write!(f, "mpeg7 backend returned invalid UTF-8: {error}"),
            Self::Xml(error) => write!(f, "failed to parse MPEG-7 XML output: {error}"),
            Self::SerializePayload(error) => {
                write!(f, "failed to serialize MPEG-7 payload JSON: {error}")
            }
        }
    }
}

impl Error for BackendError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { error, .. } => Some(error),
            Self::Spawn { error, .. } => Some(error),
            Self::InvalidUtf8(error) => Some(error),
            Self::Xml(error) => Some(error),
            Self::SerializePayload(error) => Some(error),
            Self::Unavailable(_) | Self::InvalidRequest(_) | Self::CommandFailed(_) => None,
        }
    }
}

#[derive(Debug)]
struct Runtime {
    java_bin: OsString,
    jar_path: PathBuf,
}

#[derive(Debug)]
struct Request {
    features: Vec<FeatureName>,
}

fn resolve_runtime() -> Result<Runtime, BackendError> {
    let jar_path = std::env::var_os(MPEG7_ENV_JAR)
        .map(PathBuf::from)
        .or_else(|| {
            let local = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../native/mpeg7-wrapper/MPEG7AudioEnc.jar");
            local.is_file().then_some(local)
        })
        .ok_or_else(|| {
            BackendError::Unavailable(format!(
                "set {MPEG7_ENV_JAR} to the MPEG7AudioEnc.jar path or place the jar at native/mpeg7-wrapper/MPEG7AudioEnc.jar"
            ))
        })?;

    if !jar_path.is_file() {
        return Err(BackendError::Unavailable(format!(
            "configured MPEG-7 jar does not exist: {}",
            jar_path.display()
        )));
    }

    let java_bin = std::env::var_os(MPEG7_ENV_JAVA).unwrap_or_else(|| OsString::from("java"));
    Ok(Runtime { java_bin, jar_path })
}

fn ensure_java_available(java_bin: &OsString) -> Result<(), BackendError> {
    let output = Command::new(java_bin)
        .arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| {
            BackendError::Unavailable(format!(
                "`{}` is not available: {error}",
                java_bin.to_string_lossy()
            ))
        })?;

    if output.status.success() || !output.status.success() {
        return Ok(());
    }

    Ok(())
}

fn parse_request(config_json: &str) -> Result<Request, BackendError> {
    let value: JsonValue = serde_json::from_str(config_json).map_err(|error| {
        BackendError::InvalidRequest(format!("invalid backend config JSON: {error}"))
    })?;

    let frame_level = value
        .get("features")
        .and_then(|features| features.get("frame_level"))
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);

    if frame_level {
        return Err(BackendError::InvalidRequest(
            "frame_level is not implemented for the MPEG-7 backend".to_string(),
        ));
    }

    let enabled = value
        .get("features")
        .and_then(|features| features.get("enabled"))
        .and_then(JsonValue::as_array)
        .ok_or_else(|| {
            BackendError::InvalidRequest("missing `features.enabled` array".to_string())
        })?;

    let mut features = Vec::new();
    for feature in enabled {
        let feature = feature.as_str().ok_or_else(|| {
            BackendError::InvalidRequest("feature names must be strings".to_string())
        })?;

        match feature {
            "centroid" => features.push(FeatureName::Centroid),
            "spread" => features.push(FeatureName::Spread),
            other => {
                return Err(BackendError::InvalidRequest(format!(
                    "feature `{other}` is not implemented by the current MPEG-7 wrapper"
                )));
            }
        }
    }

    Ok(Request { features })
}

fn render_config_xml(request: &Request) -> String {
    let enable_centroid_spread = request
        .features
        .iter()
        .any(|feature| matches!(feature, FeatureName::Centroid | FeatureName::Spread));

    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Config
  xmlns="http://mpeg7audioenc.sf.net/mpeg7audioenc.xsd"
  xmlns:mp7ae="http://mpeg7audioenc.sf.net/mpeg7audioenc.xsd"
  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
  xsi:schemaLocation="http://mpeg7audioenc.sf.net/mpeg7audioenc.xsd http://mpeg7audioenc.sf.net/mpeg7audioenc.xsd">
  <Module xsi:type="Resizer">
    <HopSize>10</HopSize>
  </Module>
"#,
    );

    if enable_centroid_spread {
        xml.push_str(
            r#"  <Module xsi:type="AudioSpectrumCentroidSpread" mp7ae:enable="true" />
"#,
        );
    }

    xml.push_str("</Config>\n");
    xml
}

fn write_temp_config(contents: &str) -> Result<PathBuf, BackendError> {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "audio-feature-lab-mpeg7-{}-{unique}.xml",
        process::id()
    ));
    fs::write(&path, contents).map_err(|error| BackendError::Io {
        path: path.clone(),
        error,
    })?;
    Ok(path)
}

fn build_payload_from_xml(request: &Request, xml: &str) -> Result<JsonValue, BackendError> {
    let descriptor_values = parse_descriptor_values(xml)?;

    let mut spectral = serde_json::Map::new();
    let mut warnings = Vec::new();

    for feature in &request.features {
        let (descriptor_name, output_name) = match feature {
            FeatureName::Centroid => ("AudioSpectrumCentroid", "centroid"),
            FeatureName::Spread => ("AudioSpectrumSpread", "spread"),
            _ => continue,
        };

        match descriptor_values.get(descriptor_name) {
            Some(values) if !values.is_empty() => {
                let mean = values.iter().sum::<f64>() / values.len() as f64;
                spectral.insert(output_name.to_string(), json!({ "mean": mean }));
            }
            _ => warnings.push(format!(
                "mpeg7 backend did not emit `{output_name}` for this file"
            )),
        }
    }

    let success = !spectral.is_empty();
    let status = if success {
        json!({
            "code": if warnings.is_empty() { "ok" } else { "partial" },
            "success": true,
            "warnings": warnings,
            "errors": [],
        })
    } else {
        json!({
            "code": "analysis_error",
            "success": false,
            "warnings": warnings,
            "errors": ["mpeg7 backend did not extract any requested exact descriptor"],
        })
    };

    Ok(json!({
        "audio": {},
        "features": {
            "spectral": {},
            "temporal": {},
            "rhythm": {},
            "tonal": {},
            "dynamics": {},
            "metadata": {},
            "frame_level": null,
        },
        "aggregation": {
            "spectral": spectral,
            "temporal": {},
            "rhythm": {},
            "tonal": {},
            "dynamics": {},
            "metadata": {},
        },
        "status": status,
    }))
}

fn parse_descriptor_values(xml: &str) -> Result<BTreeMap<String, Vec<f64>>, BackendError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut values: BTreeMap<String, Vec<f64>> = BTreeMap::new();

    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(BackendError::Xml)?
        {
            Event::Start(event) => {
                stack.push(local_name(event.name().as_ref()));
            }
            Event::Empty(event) => {
                stack.push(local_name(event.name().as_ref()));
                stack.pop();
            }
            Event::End(_) => {
                stack.pop();
            }
            Event::Text(text) => {
                if let Some(descriptor) = current_descriptor(&stack) {
                    let content = String::from_utf8_lossy(text.as_ref());
                    for value in parse_numbers(&content) {
                        values
                            .entry(descriptor.to_string())
                            .or_default()
                            .push(value);
                    }
                }
            }
            Event::CData(text) => {
                if let Some(descriptor) = current_descriptor(&stack) {
                    let content = String::from_utf8_lossy(text.as_ref());
                    for value in parse_numbers(&content) {
                        values
                            .entry(descriptor.to_string())
                            .or_default()
                            .push(value);
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }

        buf.clear();
    }

    Ok(values)
}

fn current_descriptor(stack: &[String]) -> Option<&str> {
    stack.iter().rev().find_map(|name| match name.as_str() {
        "AudioSpectrumCentroid" => Some("AudioSpectrumCentroid"),
        "AudioSpectrumSpread" => Some("AudioSpectrumSpread"),
        _ => None,
    })
}

fn local_name(name: &[u8]) -> String {
    let raw = String::from_utf8_lossy(name);
    raw.rsplit(':').next().unwrap_or(raw.as_ref()).to_string()
}

fn parse_numbers(text: &str) -> Vec<f64> {
    text.split(|char: char| char.is_ascii_whitespace() || char == ',')
        .filter_map(|token| {
            let token = token.trim();
            (!token.is_empty()).then_some(token)
        })
        .filter_map(|token| token.parse::<f64>().ok())
        .collect()
}

fn format_command<'a>(
    java_bin: &OsString,
    args: impl IntoIterator<Item = &'a std::ffi::OsStr>,
) -> String {
    let mut command = vec![java_bin.to_string_lossy().to_string(), "-jar".to_string()];
    command.extend(
        args.into_iter()
            .map(|arg| arg.to_string_lossy().to_string()),
    );
    command.join(" ")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        Request, build_payload_from_xml, parse_descriptor_values, parse_request, render_config_xml,
    };
    use audio_feature_lab_config::FeatureName;

    #[test]
    fn parses_descriptor_values_from_xml() {
        let values = parse_descriptor_values(
            r#"
<Mpeg7>
  <AudioSpectrumCentroid>
    <SeriesOfScalar>100.0 200.0 300.0</SeriesOfScalar>
  </AudioSpectrumCentroid>
  <AudioSpectrumSpread>
    <SeriesOfScalar>10.0 30.0</SeriesOfScalar>
  </AudioSpectrumSpread>
</Mpeg7>
"#,
        )
        .expect("xml should parse");

        assert_eq!(values["AudioSpectrumCentroid"], vec![100.0, 200.0, 300.0]);
        assert_eq!(values["AudioSpectrumSpread"], vec![10.0, 30.0]);
    }

    #[test]
    fn renders_config_for_centroid_spread_module() {
        let xml = render_config_xml(&Request {
            features: vec![FeatureName::Centroid, FeatureName::Spread],
        });

        assert!(xml.contains("AudioSpectrumCentroidSpread"));
        assert!(xml.contains("mp7ae:enable=\"true\""));
    }

    #[test]
    fn builds_partial_payload_when_one_descriptor_is_missing() {
        let payload = build_payload_from_xml(
            &Request {
                features: vec![FeatureName::Centroid, FeatureName::Spread],
            },
            r#"
<Mpeg7>
  <AudioSpectrumCentroid>
    <SeriesOfScalar>100.0 200.0 300.0</SeriesOfScalar>
  </AudioSpectrumCentroid>
</Mpeg7>
"#,
        )
        .expect("payload should build");

        assert_eq!(payload["status"]["code"], json!("partial"));
        assert_eq!(
            payload["aggregation"]["spectral"]["centroid"]["mean"],
            json!(200.0)
        );
        assert!(payload["aggregation"]["spectral"]["spread"].is_null());
    }

    #[test]
    fn rejects_non_subset_features_in_request() {
        let error = parse_request(
            r#"{
  "backend":{"name":"mpeg7"},
  "features":{"enabled":["centroid","flatness"],"frame_level":false}
}"#,
        )
        .expect_err("flatness should be rejected");

        assert!(
            error
                .to_string()
                .contains("feature `flatness` is not implemented")
        );
    }
}
