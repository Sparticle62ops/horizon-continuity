//! Neighbor-mask computation and per-method tile selection.
//!
//! The 47-tile full CTM follows the canonical connected-textures layout:
//! the orthogonal mask picks the base case (isolated, edge, outer corner,
//! straight, T-junction, surrounded), and the four diagonals refine the
//! surrounded case into the inner-corner variants, exactly as resource
//! packs that already target OptiFine's 47-tile atlas expect.

use crate::config::{Method, Rule};

/// The 8-neighborhood of one block position.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Neighbors {
    pub north: bool,
    pub south: bool,
    pub east: bool,
    pub west: bool,
    pub north_east: bool,
    pub north_west: bool,
    pub south_east: bool,
    pub south_west: bool,
}

impl Neighbors {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn all() -> Self {
        Self { north: true, south: true, east: true, west: true, north_east: true, north_west: true, south_east: true, south_west: true }
    }

    /// The 4 orthogonal bits packed as (north | south<<1 | east<<2 | west<<3).
    pub fn orthogonal_mask(&self) -> u8 {
        self.north as u8 | (self.south as u8) << 1 | (self.east as u8) << 2 | (self.west as u8) << 3
    }

    /// True when a diagonal neighbor has both of its adjacent orthogonal
    /// sides connected, so it forms an inner corner and gets its own tile.
    fn inner_corner(&self, diag: fn(&Self) -> bool, side_a: fn(&Self) -> bool, side_b: fn(&Self) -> bool) -> bool {
        diag(self) && side_a(self) && side_b(self)
    }
}

/// Tile selection for one rule at one position.
pub fn select(rule: &Rule, n: &Neighbors, seed: u64) -> usize {
    match rule.method {
        Method::Horizontal => (n.west as usize) + 2 * (n.east as usize),
        Method::Vertical => (n.north as usize) + 2 * (n.south as usize),
        Method::Random => (seed % rule.tiles.len().max(1) as u64) as usize,
        Method::CtmCompact => compact_index(n),
        Method::Ctm => full_index(n),
    }
}

/// Compact CTM: 5 tiles from the orthogonal mask.
/// 0 none | 1 edge (one side) | 2 outer corner (two adjacent) |
/// 3 straight (two opposite) | 4 center (three or four sides).
fn compact_index(n: &Neighbors) -> usize {
    match n.orthogonal_mask().count_ones() {
        0 => 0,
        1 => 1,
        2 if (n.north && n.east) || (n.east && n.south) || (n.south && n.west) || (n.west && n.north) => 2,
        2 => 3,
        _ => 4,
    }
}

/// Full 47-tile CTM.
///
/// Base index by orthogonal mask (canonical row layout):
/// isolated 0; single edges 1-4; outer corners 5-8; straights 9-10;
/// T-junctions 11-14; surrounded 15. The surrounded row (rows 16-46 in a
/// pack's atlas) is refined by inner-corner diagonals: each present inner
/// corner advances a 4-bit sub-index inside that row's 32-tile span.
fn full_index(n: &Neighbors) -> usize {
    let mask = n.orthogonal_mask();
    let base: usize = match mask {
        0b0000 => 0,
        0b0001 | 0b0010 | 0b0100 | 0b1000 => 1,
        0b0101 | 0b0110 | 0b1001 | 0b1010 => 5,
        0b0011 | 0b1100 => 9,
        0b0111 | 0b1011 | 0b1101 | 0b1110 => 11,
        0b1111 => 15,
        _ => unreachable!("4-bit mask"),
    };
    if mask != 0b1111 {
        return base;
    }
    // Surrounded: refine by the four inner-corner diagonals.
    let inner = ((n.inner_corner(|x| x.north_west, |x| x.north, |x| x.west)) as usize)
        | ((n.inner_corner(|x| x.north_east, |x| x.north, |x| x.east)) as usize) << 1
        | ((n.inner_corner(|x| x.south_east, |x| x.south, |x| x.east)) as usize) << 2
        | ((n.inner_corner(|x| x.south_west, |x| x.south, |x| x.west)) as usize) << 3;
    15 + inner
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Rule;

    fn rule(method: Method, tiles: &[&str]) -> Rule {
        Rule::new("stone", method, tiles.iter().map(|s| s.to_string()).collect())
    }

    fn n_parts(parts: &[&str]) -> Neighbors {
        let mut n = Neighbors::none();
        for p in parts {
            match *p {
                "n" => n.north = true,
                "s" => n.south = true,
                "e" => n.east = true,
                "w" => n.west = true,
                "ne" => n.north_east = true,
                "nw" => n.north_west = true,
                "se" => n.south_east = true,
                "sw" => n.south_west = true,
                _ => {}
            }
        }
        n
    }

    #[test]
    fn horizontal_four_tile_layout() {
        let r = rule(Method::Horizontal, &["h0", "h1", "h2", "h3"]);
        // tile order: none, west only, east only, both.
        assert_eq!(select(&r, &n_parts(&[]), 0), 0);
        assert_eq!(select(&r, &n_parts(&["w"]), 0), 1);
        assert_eq!(select(&r, &n_parts(&["e"]), 0), 2);
        assert_eq!(select(&r, &n_parts(&["e", "w"]), 0), 3);
        // North/south neighbors do not affect horizontal connect.
        assert_eq!(select(&r, &n_parts(&["n", "s"]), 0), 0);
    }

    #[test]
    fn vertical_four_tile_layout() {
        let r = rule(Method::Vertical, &["v0", "v1", "v2", "v3"]);
        assert_eq!(select(&r, &n_parts(&["n"]), 0), 1);
        assert_eq!(select(&r, &n_parts(&["s"]), 0), 2);
        assert_eq!(select(&r, &n_parts(&["n", "s"]), 0), 3);
        assert_eq!(select(&r, &n_parts(&["e"]), 0), 0);
    }

    #[test]
    fn compact_five_tile_cases() {
        let r = rule(Method::CtmCompact, &["c0", "c1", "c2", "c3", "c4"]);
        assert_eq!(select(&r, &n_parts(&[]), 0), 0);
        assert_eq!(select(&r, &n_parts(&["e"]), 0), 1);
        assert_eq!(select(&r, &n_parts(&["n", "e"]), 0), 2); // adjacent -> corner
        assert_eq!(select(&r, &n_parts(&["n", "s"]), 0), 3); // opposite -> straight
        assert_eq!(select(&r, &n_parts(&["n", "e", "s"]), 0), 4);
        assert_eq!(select(&r, &n_parts(&["n", "s", "e", "w"]), 0), 4);
    }

    #[test]
    fn full_ctm_base_cases() {
        let r = rule(Method::Ctm, &["t0", "t1", "t5", "t9", "t11", "t15", "t16"]);
        assert_eq!(select(&r, &n_parts(&[]), 0), 0);
        assert_eq!(select(&r, &n_parts(&["e"]), 0), 1);
        assert_eq!(select(&r, &n_parts(&["n", "e"]), 0), 5); // outer corner
        assert_eq!(select(&r, &n_parts(&["n", "s"]), 0), 9); // straight
        assert_eq!(select(&r, &n_parts(&["n", "s", "e"]), 0), 11); // T
        assert_eq!(select(&r, &n_parts(&["n", "s", "e", "w"]), 0), 15); // surrounded
        // No inner corners while surrounded: row start (16 in the pack).
        assert_eq!(select(&r, &n_parts(&["n", "s", "e", "w"]), 0), 15); // no diagonals
    }

    #[test]
    fn full_ctm_inner_corner_refinement() {
        let r = rule(Method::Ctm, &["t0", "t1", "t5", "t9", "t11", "t15", "t16", "t17", "t18"]);
        // Surrounded + NW inner corner only -> sub-index 1.
        assert_eq!(select(&r, &n_parts(&["n", "s", "e", "w", "nw"]), 0), 16);
        // NW + SE -> sub-index 1 | 4 = 5 -> 15 + 5 = 20.
        assert_eq!(select(&r, &n_parts(&["n", "s", "e", "w", "nw", "se"]), 0), 20);
        // With all four sides connected, a present NE diagonal is an inner corner.
        assert_eq!(select(&r, &n_parts(&["n", "s", "e", "w", "ne"]), 0), 17);
    }

    #[test]
    fn random_is_seeded_and_bounded() {
        let r = rule(Method::Random, &["r0", "r1", "r2"]);
        assert_eq!(select(&r, &Neighbors::none(), 7), (7 % 3) as usize);
        assert_eq!(select(&r, &Neighbors::none(), 9), 0);
    }

    #[test]
    fn neighbors_helpers() {
        assert_eq!(Neighbors::none().orthogonal_mask(), 0);
        assert_eq!(Neighbors::all().orthogonal_mask(), 0b1111);
    }
}
