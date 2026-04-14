use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigLayer {
    RuntimeFlag,
    Project,
    User,
    Default,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectiveConfigEntry {
    pub key: String,
    pub required_value: Value,
    pub effective_value: Value,
    pub source_layer: ConfigLayer,
    pub status: CheckStatus,
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectiveConfigReport {
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub generated_at: Option<OffsetDateTime>,
    pub repo_root: Option<PathBuf>,
    pub entries: Vec<EffectiveConfigEntry>,
}

impl EffectiveConfigReport {
    #[must_use]
    pub fn overall_status(&self) -> CheckStatus {
        self.entries
            .iter()
            .map(|entry| entry.status)
            .max()
            .unwrap_or(CheckStatus::Pass)
    }

    #[must_use]
    pub fn failing_entries(&self) -> Vec<&EffectiveConfigEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.status == CheckStatus::Fail)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoctorFinding {
    pub check: String,
    pub status: CheckStatus,
    pub message: String,
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoctorReport {
    pub repo_root: PathBuf,
    pub findings: Vec<DoctorFinding>,
    pub effective_config: EffectiveConfigReport,
}

impl DoctorReport {
    #[must_use]
    pub fn overall_status(&self) -> CheckStatus {
        let findings_status = self
            .findings
            .iter()
            .map(|finding| finding.status)
            .max()
            .unwrap_or(CheckStatus::Pass);

        findings_status.max(self.effective_config.overall_status())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QualificationStatus {
    Pass,
    Skipped,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualificationGateResult {
    pub gate: String,
    pub status: QualificationStatus,
    pub message: String,
    pub evidence_path: Option<PathBuf>,
    pub skipped_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualificationReport {
    pub codex_build: String,
    #[serde(with = "time::serde::rfc3339")]
    pub tested_at: OffsetDateTime,
    pub gates: Vec<QualificationGateResult>,
    pub evidence_root: PathBuf,
}

impl QualificationReport {
    #[must_use]
    pub fn overall_status(&self) -> QualificationStatus {
        self.gates
            .iter()
            .map(|gate| gate.status)
            .max()
            .unwrap_or(QualificationStatus::Pass)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::{
        CheckStatus, ConfigLayer, DoctorFinding, DoctorReport, EffectiveConfigEntry,
        EffectiveConfigReport,
    };

    #[test]
    fn report_rolls_up_failure_status() {
        let report = DoctorReport {
            repo_root: PathBuf::from("/repo"),
            findings: vec![DoctorFinding {
                check: "trusted repo state".to_owned(),
                status: CheckStatus::Warn,
                message: "repo must be trusted".to_owned(),
                remediation: Some("trust the repo".to_owned()),
            }],
            effective_config: EffectiveConfigReport {
                generated_at: None,
                repo_root: Some(PathBuf::from("/repo")),
                entries: vec![EffectiveConfigEntry {
                    key: "features.codex_hooks".to_owned(),
                    required_value: json!(true),
                    effective_value: json!(false),
                    source_layer: ConfigLayer::User,
                    status: CheckStatus::Fail,
                    remediation: Some("set features.codex_hooks = true".to_owned()),
                }],
            },
        };

        assert_eq!(report.overall_status(), CheckStatus::Fail);
        assert_eq!(report.effective_config.failing_entries().len(), 1);
    }
}
