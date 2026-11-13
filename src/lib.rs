//! Continuity for Horizon (behavioral clean-room port of the LGPL-3.0 mod).
//!
//! Upstream lets resource packs define OptiFine-style connected-texture rules
//! and renders connected textures across neighboring blocks. The resource-pack
//! loading and the atlas/render pass are host seams (see README); this crate
//! carries the policy logic: CTM rule parsing, neighbor-bitmask computation,
//! and per-method tile selection.

pub mod config;
pub mod ctm;

/// Which neighbor directions a rule consults, mirroring upstream's
/// `connectivity` values (full 8-neighborhood, or the orthogonal 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connectivity {
    Full,
    Orthogonal,
}

/// Per-rule runtime state: parsed rules for one resource pack.
#[derive(Debug, Clone, Default)]
pub struct ContinuityState {
    pub rules: Vec<config::Rule>,
}

impl ContinuityState {
    pub fn load(rules: Vec<config::Rule>) -> Self {
        Self { rules }
    }

    /// The tile index for one block position given its neighbors: the first
    /// rule whose blockstate matches wins, then the method's own selection.
    pub fn tile_for(
        &self,
        block: &str,
        neighbors: &ctm::Neighbors,
        seed: u64,
    ) -> Option<String> {
        let rule = self.rules.iter().find(|r| r.matches(block))?;
        let idx = ctm::select(rule, neighbors, seed);
        Some(rule.tile(idx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::Method;

    fn rule(method: Method, tiles: &[&str]) -> config::Rule {
        config::Rule::new("stone", method, tiles.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn first_matching_rule_wins() {
        let st = ContinuityState::load(vec![
            rule(Method::Horizontal, &["a", "b", "c", "d"]),
            rule(Method::Random, &["r0", "r1"]),
        ]);
        let n = ctm::Neighbors::none();
        let t = st.tile_for("stone", &n, 0).unwrap();
        // Horizontal with no neighbors picks the unconnected variant (tile 0).
        assert_eq!(t, "a");
    }

    #[test]
    fn no_rule_means_no_override() {
        let st = ContinuityState::load(vec![rule(Method::Random, &["r0", "r1"])]);
        assert!(st.tile_for("dirt", &ctm::Neighbors::none(), 0).is_none());
    }

    #[test]
    fn blockstate_match_is_exact() {
        let mut r = rule(Method::Random, &["r0"]);
        r.blockstates.push("granite".to_string());
        let st = ContinuityState::load(vec![r]);
        assert!(st.tile_for("granite", &ctm::Neighbors::none(), 0).is_some());
        assert!(st.tile_for("granite_stairs", &ctm::Neighbors::none(), 0).is_none());
    }

    #[test]
    fn deterministic_seed_for_random() {
        let st = ContinuityState::load(vec![rule(Method::Random, &["r0", "r1", "r2"])]);
        let n = ctm::Neighbors::none();
        assert_eq!(st.tile_for("stone", &n, 42), st.tile_for("stone", &n, 42));
    }
}
