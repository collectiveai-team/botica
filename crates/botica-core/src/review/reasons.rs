//! The codes of the reasons that a classification and a risk assessment record.

use serde::{Deserialize, Serialize};

/// Why a classification blocks automation or routes a change to a person (ADR 0018).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReasonCode {
    IncompleteRisk,
    ReviewEvidence,
    CheckEvidence,
    RequiredChecks,
    UnresolvedFinding,
    MissingAcceptanceContext,
    ContextChanged,
    ContextUnavailable,
    ContextLimit,
    ContextByteLimit,
    CriticalArea,
    RiskNotLow,
    EvidenceChanged,
    ClassifierUnavailable,
    Decision,
    Unverifiable,
    ImpactUsers,
    LowConfidence,
}

impl ReasonCode {
    /// The code of a recorded reason; a code this release does not know is `None`.
    pub fn parse(code: &str) -> Option<Self> {
        serde_json::from_value(serde_json::Value::from(code)).ok()
    }

    /// Whether Jev reports this reason from its probabilities.
    pub const fn from_model(self) -> bool {
        matches!(
            self,
            Self::Decision | Self::Unverifiable | Self::ImpactUsers | Self::LowConfidence
        )
    }

    /// Whether a model reason raises the deterministic tier one step.
    pub const fn raises_risk(self) -> bool {
        matches!(
            self,
            Self::Decision | Self::Unverifiable | Self::ImpactUsers
        )
    }

    /// Whether the reason says the acceptance or change context was incomplete.
    pub const fn context_incomplete(self) -> bool {
        match self {
            Self::MissingAcceptanceContext
            | Self::ContextUnavailable
            | Self::ContextLimit
            | Self::ContextByteLimit
            | Self::ContextChanged => true,
            Self::IncompleteRisk
            | Self::ReviewEvidence
            | Self::CheckEvidence
            | Self::RequiredChecks
            | Self::UnresolvedFinding
            | Self::CriticalArea
            | Self::RiskNotLow
            | Self::EvidenceChanged
            | Self::ClassifierUnavailable
            | Self::Decision
            | Self::Unverifiable
            | Self::ImpactUsers
            | Self::LowConfidence => false,
        }
    }

    /// Whether the model locates this reason in the changed files.
    pub const fn localized(self) -> bool {
        matches!(self, Self::Decision | Self::Unverifiable)
    }
}

/// Why a branch or integration assessment is incomplete (`schema/review-tier.schema.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RiskReasonCode {
    MissingHistory,
    NoCommonAncestor,
    MergeConflict,
    UnsupportedIntegrationMethod,
    InvalidConfig,
    UnreadableDiff,
    UnsupportedDetector,
    IntegrationFailed,
}

impl RiskReasonCode {
    pub const ALL: [Self; 8] = [
        Self::MissingHistory,
        Self::NoCommonAncestor,
        Self::MergeConflict,
        Self::UnsupportedIntegrationMethod,
        Self::InvalidConfig,
        Self::UnreadableDiff,
        Self::UnsupportedDetector,
        Self::IntegrationFailed,
    ];
}
