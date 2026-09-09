/// The closed built-in point-size vocabulary and its opt-in warning policy
/// (§FS-list.3.4, §FS-config.3.1). These config-facing types stay with the
/// parser so the general model remains reserved for scan and declaration data.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PointSizeUnit {
    Lines,
    Words,
    Bytes,
}

impl PointSizeUnit {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lines => "lines",
            Self::Words => "words",
            Self::Bytes => "bytes",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "lines" => Some(Self::Lines),
            "words" => Some(Self::Words),
            "bytes" => Some(Self::Bytes),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LeadSizeWarning {
    pub max: usize,
    pub unit: PointSizeUnit,
}

/// Parse the one inline table in grund's line-oriented config surface
/// (§FS-config.3.1). Keeping this parser specific makes duplicate, missing, and
/// extra fields loud without silently widening the rest of the TOML subset.
fn parse_lead_size_warning(path: &Path, line: usize, value: &str) -> Result<LeadSizeWarning> {
    if !(value.starts_with('{') && value.ends_with('}')) {
        return bail_config(
            path,
            line,
            "lead_size_warning must be `{ max = <N>, unit = \"lines|words|bytes\" }`"
                .to_string(),
        );
    }
    let inner = value[1..value.len() - 1].trim();
    let mut max = None;
    let mut unit = None;
    for field in inner.split(',') {
        let Some((key, raw)) = field.split_once('=') else {
            return bail_config(path, line, "invalid lead_size_warning field".to_string());
        };
        let key = key.trim();
        let raw = raw.trim();
        match key {
            "max" => {
                if max.is_some() {
                    return bail_config(
                        path,
                        line,
                        "duplicate lead_size_warning field `max`".to_string(),
                    );
                }
                max = Some(parse_usize(path, line, raw).map_err(|_| {
                    anyhow!(
                        "{}:{}: lead_size_warning max must be a non-negative integer",
                        format_path(path),
                        line
                    )
                })?);
            }
            "unit" => {
                if unit.is_some() {
                    return bail_config(
                        path,
                        line,
                        "duplicate lead_size_warning field `unit`".to_string(),
                    );
                }
                let raw_unit = parse_string(path, line, raw).map_err(|_| {
                    anyhow!(
                        "{}:{}: lead_size_warning unit must be a string",
                        format_path(path),
                        line
                    )
                })?;
                unit = PointSizeUnit::parse(&raw_unit);
                if unit.is_none() {
                    return bail_config(
                        path,
                        line,
                        format!(
                            "unknown lead_size_warning unit `{raw_unit}` (expected lines, words, or bytes)"
                        ),
                    );
                }
            }
            "" => {
                return bail_config(path, line, "invalid lead_size_warning field".to_string());
            }
            other => {
                return bail_config(
                    path,
                    line,
                    format!("unknown lead_size_warning field `{other}`"),
                );
            }
        }
    }
    let Some(max) = max else {
        return bail_config(
            path,
            line,
            "lead_size_warning requires field `max`".to_string(),
        );
    };
    let Some(unit) = unit else {
        return bail_config(
            path,
            line,
            "lead_size_warning requires field `unit`".to_string(),
        );
    };
    Ok(LeadSizeWarning { max, unit })
}
