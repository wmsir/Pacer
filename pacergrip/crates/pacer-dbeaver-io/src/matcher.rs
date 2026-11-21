use serde::{Deserialize, Serialize};

use pacer_core::{ConnectionMode, ConnectionProfile, EnvLevel};

/// Specificity level of a fingerprint match between an imported profile and a Pacer instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MatchKind {
    ExactHostPortDb,
    HostPortOnly,
    NameOnly,
    Unmatched,
}

/// Result of matching a locally imported profile against Pacer-managed instances.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchResult {
    pub local_profile_id: String,
    pub local_name: String,
    pub match_kind: MatchKind,
    pub matched_pacer_instance_id: Option<i64>,
    pub matched_pacer_name: Option<String>,
    pub matched_env: Option<EnvLevel>,
    pub upgrade_recommended: bool,
}

/// Matches locally imported connections against Pacer-managed profiles by fingerprint & name.
pub fn match_profiles_with_pacer(
    local: &[ConnectionProfile],
    pacer_profiles: &[ConnectionProfile],
) -> Vec<MatchResult> {
    local
        .iter()
        .map(|loc| find_best_match(loc, pacer_profiles))
        .collect()
}

fn find_best_match(local: &ConnectionProfile, pacer_profiles: &[ConnectionProfile]) -> MatchResult {
    let mut best_kind = MatchKind::Unmatched;
    let mut best_candidate: Option<&ConnectionProfile> = None;

    for candidate in pacer_profiles {
        let kind = if local
            .fingerprint
            .matches_exact_host_port_db(&candidate.fingerprint)
        {
            MatchKind::ExactHostPortDb
        } else if local.fingerprint.matches_host_port(&candidate.fingerprint) {
            MatchKind::HostPortOnly
        } else if !local.name.trim().is_empty()
            && local.name.trim().eq_ignore_ascii_case(candidate.name.trim())
        {
            MatchKind::NameOnly
        } else {
            continue;
        };

        if match_rank(kind) > match_rank(best_kind) {
            best_kind = kind;
            best_candidate = Some(candidate);
            if best_kind == MatchKind::ExactHostPortDb {
                break;
            }
        }
    }

    let matched_pacer_instance_id = best_candidate.and_then(|c| match &c.mode {
        ConnectionMode::PacerManaged { instance_id, .. }
        | ConnectionMode::Hybrid {
            pacer_instance_id: instance_id,
            ..
        } => Some(*instance_id),
        ConnectionMode::Direct(_) => None,
    });

    MatchResult {
        local_profile_id: local.id.clone(),
        local_name: local.name.clone(),
        match_kind: best_kind,
        matched_pacer_instance_id,
        matched_pacer_name: best_candidate.map(|c| c.name.clone()),
        matched_env: best_candidate.map(|c| c.env),
        upgrade_recommended: matches!(
            best_kind,
            MatchKind::ExactHostPortDb | MatchKind::HostPortOnly
        ),
    }
}

fn match_rank(kind: MatchKind) -> u8 {
    match kind {
        MatchKind::ExactHostPortDb => 3,
        MatchKind::HostPortOnly => 2,
        MatchKind::NameOnly => 1,
        MatchKind::Unmatched => 0,
    }
}
