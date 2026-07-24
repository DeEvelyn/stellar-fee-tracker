use std::path::PathBuf;

use crate::error::DevkitError;

/// Supported file formats for conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    Csv,
    Json,
}

impl FileFormat {
    /// Parse a format string into a `FileFormat`.
    pub fn from_str(s: &str) -> Result<Self, DevkitError> {
        match s.to_lowercase().as_str() {
            "csv" => Ok(FileFormat::Csv),
            "json" => Ok(FileFormat::Json),
            other => Err(DevkitError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("unsupported format: {other}"),
            ))),
        }
    }
}

/// Configuration for a convert operation.
#[derive(Debug, Clone)]
pub struct ConvertConfig {
    /// Path to the input file.
    pub input: PathBuf,
    /// Source format.
    pub from: FileFormat,
    /// Destination format.
    pub to: FileFormat,
    /// Path to the output file.
    pub output: PathBuf,
}

/// Perform the conversion described by `config`.
pub fn convert(config: &ConvertConfig) -> Result<(), DevkitError> {
    let raw = std::fs::read_to_string(&config.input)?;

    let content = match (config.from, config.to) {
        (FileFormat::Csv, FileFormat::Json) => csv_to_json(&raw)?,
        (FileFormat::Json, FileFormat::Csv) => json_to_csv(&raw)?,
        (FileFormat::Csv, FileFormat::Csv) | (FileFormat::Json, FileFormat::Json) => raw,
    };

    std::fs::write(&config.output, content)?;
    Ok(())
}

/// Convert CSV fee data to JSON.
fn csv_to_json(csv: &str) -> Result<String, DevkitError> {
    let mut lines = csv.lines();
    let header_line = lines.next().unwrap_or_default();
    let headers: Vec<&str> = header_line.split(',').map(|h| h.trim()).collect();

    let mut records: Vec<serde_json::Value> = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let values: Vec<&str> = line.split(',').map(|v| v.trim()).collect();
        let mut map = serde_json::Map::new();
        for (i, header) in headers.iter().enumerate() {
            let val = values.get(i).copied().unwrap_or("");
            let json_val = if val.eq_ignore_ascii_case("true") {
                serde_json::Value::Bool(true)
            } else if val.eq_ignore_ascii_case("false") {
                serde_json::Value::Bool(false)
            } else {
                val.parse::<u64>()
                    .map(|n| {
                        serde_json::Value::Number(
                            serde_json::Number::from(n),
                        )
                    })
                    .or_else(|_| {
                        val.parse::<f64>().map(|f| {
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(f)
                                    .unwrap_or_else(|| serde_json::Number::from(0)),
                            )
                        })
                    })
                    .unwrap_or_else(|_| serde_json::Value::String(val.to_string()))
            };
            map.insert(header.to_string(), json_val);
        }
        records.push(serde_json::Value::Object(map));
    }

    serde_json::to_string_pretty(&records).map_err(|e| DevkitError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
}

/// Convert JSON fee data array to CSV.
fn json_to_csv(json: &str) -> Result<String, DevkitError> {
    let records: Vec<serde_json::Value> =
        serde_json::from_str(json).map_err(|e| DevkitError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;

    if records.is_empty() {
        return Ok(String::new());
    }

    let first = &records[0];
    let object = match first.as_object() {
        Some(o) => o,
        None => return Ok(String::new()),
    };

    let headers: Vec<&str> = object.keys().map(|s| s.as_str()).collect();
    let mut out = headers.join(",");
    out.push('\n');

    for record in &records {
        let empty_map = serde_json::Map::new();
        let obj = record.as_object().unwrap_or(&empty_map);
        let row: Vec<String> = headers
            .iter()
            .map(|h| match obj.get(*h) {
                Some(serde_json::Value::String(s)) => s.clone(),
                Some(v) => v.to_string(),
                None => String::new(),
            })
            .collect();
        out.push_str(&row.join(","));
        out.push('\n');
    }

    Ok(out)
}

/// CLI entry point for the `convert` subcommand.
pub fn run(args: &[String]) -> Result<(), DevkitError> {
    let mut input: Option<PathBuf> = None;
    let mut from: Option<String> = None;
    let mut to: Option<String> = None;
    let mut output: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--file" => {
                i += 1;
                input = Some(PathBuf::from(args.get(i).ok_or_else(|| {
                    DevkitError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "missing value for --file",
                    ))
                })?));
            }
            "--from" => {
                i += 1;
                from = Some(args.get(i).ok_or_else(|| {
                    DevkitError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "missing value for --from",
                    ))
                })?.clone());
            }
            "--to" => {
                i += 1;
                to = Some(args.get(i).ok_or_else(|| {
                    DevkitError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "missing value for --to",
                    ))
                })?.clone());
            }
            "--output" => {
                i += 1;
                output = Some(PathBuf::from(args.get(i).ok_or_else(|| {
                    DevkitError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "missing value for --output",
                    ))
                })?));
            }
            other => {
                return Err(DevkitError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown argument: {other}"),
                )));
            }
        }
        i += 1;
    }

    let input = input.ok_or_else(|| {
        DevkitError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--file is required",
        ))
    })?;
    let from_str = from.ok_or_else(|| {
        DevkitError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--from is required",
        ))
    })?;
    let to_str = to.ok_or_else(|| {
        DevkitError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--to is required",
        ))
    })?;
    let output = output.ok_or_else(|| {
        DevkitError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--output is required",
        ))
    })?;

    let config = ConvertConfig {
        input,
        from: FileFormat::from_str(&from_str)?,
        to: FileFormat::from_str(&to_str)?,
        output,
    };

    convert(&config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_to_json_roundtrip() {
        let csv = "timestamp,fee,ledger,is_spike\n1000,100,1,false\n1005,1000,2,true\n";
        let json = csv_to_json(csv).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["timestamp"], 1000);
        assert_eq!(arr[1]["fee"], 1000);
        assert_eq!(arr[1]["is_spike"], true);
    }

    #[test]
    fn json_to_csv_basic() {
        let json = r#"[{"timestamp":1000,"fee":100,"ledger":1},{"timestamp":1005,"fee":200,"ledger":2}]"#;
        let csv = json_to_csv(json).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("timestamp"));
        assert!(lines[1].contains("100"));
    }

    #[test]
    fn parse_format() {
        assert_eq!(FileFormat::from_str("csv").unwrap(), FileFormat::Csv);
        assert_eq!(FileFormat::from_str("JSON").unwrap(), FileFormat::Json);
        assert!(FileFormat::from_str("xml").is_err());
    }
}
