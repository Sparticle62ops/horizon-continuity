//! CTM rule parsing: OptiFine-style `.properties` rule files, the format
//! resource packs already ship (matchBlocks / method / tiles / layer).
//!
//! Behavioral contract: line-based key=value, `#` comments, `tiles` as
//! comma/space separated names or numeric ranges (`0-3`), `method` one of
//! `ctm`, `ctm_compact`, `horizontal`, `vertical`, `random`. Unknown keys and
//! blank lines are ignored, so packs keep loading on a newer format.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Full 47-tile CTM (8-neighborhood, inner corners).
    Ctm,
    /// Compact 5-tile CTM (orthogonal neighbors only).
    CtmCompact,
    /// 4-tile horizontal connect along X.
    Horizontal,
    /// 4-tile vertical connect along Z.
    Vertical,
    /// One tile chosen per position, deterministic in the position seed.
    Random,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// Blocks this rule applies to (exact ids).
    pub blockstates: Vec<String>,
    pub method: Method,
    pub tiles: Vec<String>,
}

impl Rule {
    pub fn new(block: &str, method: Method, tiles: Vec<String>) -> Self {
        Self { blockstates: vec![block.to_string()], method, tiles }
    }

    pub fn matches(&self, block: &str) -> bool {
        self.blockstates.iter().any(|b| b == block)
    }

    /// The tile name for a selection index, saturating on out-of-range so a
    /// short tile list degrades to its last entry instead of panicking.
    pub fn tile(&self, idx: usize) -> String {
        self.tiles[idx.min(self.tiles.len() - 1)].clone()
    }

    /// Parse one rule file's text; returns None when it carries no method or
    /// no blocks (not a CTM rule).
    pub fn parse(text: &str) -> Option<Self> {
        let mut method = None;
        let mut blocks = Vec::new();
        let mut tiles = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line.split_once('=')?;
            let value = value.trim();
            match key.trim() {
                "method" => method = Some(match value {
                    "ctm" => Method::Ctm,
                    "ctm_compact" => Method::CtmCompact,
                    "horizontal" => Method::Horizontal,
                    "vertical" => Method::Vertical,
                    "random" => Method::Random,
                    _ => continue,
                }),
                "matchBlocks" | "matchblock" | "block" => {
                    blocks.extend(value.split(|c: char| c == ',' || c == ' ').filter(|s| !s.is_empty()).map(|s| s.to_string()))
                }
                "tiles" | "tile" => {
                    for tok in value.split(|c: char| c == ',' || c == ' ').filter(|s| !s.is_empty()) {
                        if let Some((a, b)) = tok.split_once('-') {
                            if let (Ok(a), Ok(b)) = (a.trim().parse::<u32>(), b.trim().parse::<u32>()) {
                                for n in a..=b {
                                    tiles.push(n.to_string());
                                }
                                continue;
                            }
                        }
                        tiles.push(tok.to_string());
                    }
                }
                _ => {}
            }
        }
        if blocks.is_empty() || tiles.is_empty() {
            return None;
        }
        Some(Self {
            blockstates: blocks,
            method: method.unwrap_or(Method::Ctm),
            tiles,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_optifine_style_properties() {
        let r = Rule::parse(
            "# glass rules\nmatchBlocks=glass\nmethod=ctm_compact\ntiles=0-4\n",
        )
        .unwrap();
        assert_eq!(r.blockstates, ["glass"]);
        assert_eq!(r.method, Method::CtmCompact);
        assert_eq!(r.tiles, ["0", "1", "2", "3", "4"]);
    }

    #[test]
    fn named_tiles_split_on_commas_and_spaces() {
        let r = Rule::parse("matchBlocks=stone\nmethod=random\ntiles=a b, c\n").unwrap();
        assert_eq!(r.tiles, ["a", "b", "c"]);
    }

    #[test]
    fn missing_method_defaults_to_full_ctm() {
        let r = Rule::parse("matchBlocks=stone\ntiles=t0\n").unwrap();
        assert_eq!(r.method, Method::Ctm);
    }

    #[test]
    fn no_blocks_or_no_tiles_is_not_a_rule() {
        assert!(Rule::parse("method=horizontal\ntiles=0\n").is_none());
        assert!(Rule::parse("matchBlocks=stone\nmethod=horizontal\n").is_none());
    }

    #[test]
    fn unknown_method_and_keys_are_ignored() {
        let r = Rule::parse("matchBlocks=stone\nmethod=holographic\ntiles=x\nlayer=0\n").unwrap();
        assert_eq!(r.method, Method::Ctm);
        assert_eq!(r.tiles, ["x"]);
    }

    #[test]
    fn tile_saturates_on_short_list() {
        let r = Rule::new("stone", Method::Horizontal, vec!["a".into(), "b".into(), "c".into(), "d".into()]);
        assert_eq!(r.tile(3), "d");
        let short = Rule::new("stone", Method::Horizontal, vec!["only".into()]);
        assert_eq!(short.tile(3), "only");
    }
}
