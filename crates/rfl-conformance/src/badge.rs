// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Derive a conformance **badge** from a certificate (`spec/05` § Fidelity tier
//! and the badge). The badge records, per action, the envelope class and the
//! achieved fidelity tier, and rolls up to the embodiment-level honesty summary:
//! the **regime tier** (who verified) and the **achieved fidelity** (no higher
//! than the weakest confirmed action), plus whether the `RFL™` trademark is
//! permitted (`spec/05` § The trademark gate: Tier 2 / Tier 3 only).
//!
//! A certificate emitted by `rfl certify` is **Tier 1** (self-certification) by
//! construction, so its badge never permits the trademark. The derivation is a
//! pure read of the certificate JSON — it changes nothing the `content_hash`
//! covers.

use anyhow::{Context, Result};

/// One action's badge line.
pub struct ActionBadge {
    /// The correlated action id.
    pub action_id: String,
    /// The envelope class verified (absent for perception actions).
    pub envelope_class: Option<String>,
    /// The fidelity tier achieved (absent for actions with no confirmation tier).
    pub fidelity_tier: Option<String>,
    /// Whether every obligation passed for this action.
    pub passed: bool,
}

/// The rolled-up badge for a certificate.
pub struct Badge {
    /// The certified skill id.
    pub skill: String,
    /// The certified embodiment id.
    pub embodiment: String,
    /// `"pass"` / `"fail"` from the certificate.
    pub result: String,
    /// The regime tier (who verified): 1 = self-certification (`rfl certify`).
    pub regime_tier: u8,
    /// The achieved fidelity tier — the **weakest** confirmed action's tier, so
    /// the badge never over-claims (`None` when no action carried a tier).
    pub achieved_fidelity: Option<String>,
    /// Whether the `RFL™` trademark is permitted (regime Tier 2 / Tier 3 only).
    pub trademark_permitted: bool,
    /// Per-action badge lines, in certificate order.
    pub actions: Vec<ActionBadge>,
}

/// Fidelity ordering: `proxy_reactive` < `proxy` < `manifold`. Unknown/None sorts
/// above nothing (it is filtered out before ranking).
fn fidelity_rank(tier: &str) -> u8 {
    match tier {
        "manifold" => 3,
        "proxy" => 2,
        "proxy_reactive" => 1,
        _ => 0,
    }
}

/// Derive a [`Badge`] from a certificate's JSON text (the `rfl certify` / `rfl
/// sign` output). Reads only; does not validate the content hash (`rfl verify`
/// does that).
///
/// # Errors
/// Returns an error if the certificate is not parseable JSON.
pub fn derive_badge(cert_json: &str) -> Result<Badge> {
    let v: serde_json::Value = serde_json::from_str(cert_json).context("parse certificate JSON")?;

    let id_of = |key: &str| {
        v.get(key)
            .and_then(|s| s.get("id"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let result = v
        .get("result")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string();

    let mut actions = Vec::new();
    if let Some(arr) = v.get("actions").and_then(serde_json::Value::as_array) {
        for a in arr {
            let s = |key: &str| {
                a.get(key)
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            };
            actions.push(ActionBadge {
                action_id: s("action_id").unwrap_or_default(),
                envelope_class: s("envelope_class"),
                fidelity_tier: s("fidelity_tier"),
                passed: a
                    .get("passed")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            });
        }
    }

    // The achieved fidelity is the weakest confirmed action's tier: the badge
    // cannot honestly claim `manifold` if any confirmed action degraded.
    let achieved_fidelity = actions
        .iter()
        .filter_map(|a| a.fidelity_tier.as_deref())
        .min_by_key(|t| fidelity_rank(t))
        .map(str::to_string);

    // `rfl certify` is self-certification (Tier 1). Tier 2 / Tier 3 are
    // steward / independent verification, out of scope for the self-cert tool.
    let regime_tier = 1u8;
    let trademark_permitted = matches!(regime_tier, 2 | 3);

    Ok(Badge {
        skill: id_of("skill"),
        embodiment: id_of("embodiment"),
        result,
        regime_tier,
        achieved_fidelity,
        trademark_permitted,
        actions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(rel: &str) -> String {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples")
            .join(rel);
        std::fs::read_to_string(p).unwrap()
    }

    #[test]
    fn manifold_certificate_badges_manifold_tier1_no_trademark() {
        let b = derive_badge(&fixture("01-cable-insertion/certificate.json")).unwrap();
        assert_eq!(b.skill, "cable-insertion");
        assert_eq!(b.embodiment, "wonik-allegro-v4");
        assert_eq!(b.result, "pass");
        assert_eq!(b.regime_tier, 1);
        assert_eq!(b.achieved_fidelity.as_deref(), Some("manifold"));
        assert!(
            !b.trademark_permitted,
            "Tier 1 must not permit the trademark"
        );
        assert!(!b.actions.is_empty());
    }

    #[test]
    fn proxy_certificate_rolls_up_to_proxy_not_manifold() {
        // The no-tactile pneumatic hand degrades to the force/position proxy; the
        // badge must report the weakest tier (proxy), never manifold.
        let b = derive_badge(&fixture("01-cable-insertion/certificate-pneumatic.json")).unwrap();
        assert_eq!(b.achieved_fidelity.as_deref(), Some("proxy"));
        assert!(!b.trademark_permitted);
    }

    #[test]
    fn malformed_json_errors() {
        assert!(derive_badge("{not json").is_err());
    }
}
