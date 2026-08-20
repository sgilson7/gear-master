/// A polyomino, stored as cell offsets from the piece's anchor and always
/// normalized so the smallest x and the smallest y are both 0.
///
/// Normalization is what lets the anchor of a placed piece be recovered as
/// "the min x and min y of the cells it occupies" — no separate bookkeeping,
/// so the grid and the anchor can never disagree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shape {
    cells: Vec<(i8, i8)>,
}

impl Shape {
    pub fn new(cells: &[(i8, i8)]) -> Self {
        let mut s = Shape { cells: cells.to_vec() };
        s.normalize();
        s
    }

    fn normalize(&mut self) {
        let min_x = self.cells.iter().map(|c| c.0).min().unwrap_or(0);
        let min_y = self.cells.iter().map(|c| c.1).min().unwrap_or(0);
        for c in &mut self.cells {
            c.0 -= min_x;
            c.1 -= min_y;
        }
        self.cells.sort_unstable();
        self.cells.dedup();
    }

    pub fn cells(&self) -> &[(i8, i8)] {
        &self.cells
    }

    pub fn area(&self) -> usize {
        self.cells.len()
    }

    pub fn width(&self) -> u8 {
        self.cells.iter().map(|c| c.0).max().map(|m| m as u8 + 1).unwrap_or(0)
    }

    pub fn height(&self) -> u8 {
        self.cells.iter().map(|c| c.1).max().map(|m| m as u8 + 1).unwrap_or(0)
    }

    /// One quarter turn clockwise: `(x, y) -> (-y, x)`, re-normalized.
    pub fn rotated_cw(&self) -> Shape {
        let cells: Vec<(i8, i8)> = self.cells.iter().map(|&(x, y)| (-y, x)).collect();
        Shape::new(&cells)
    }

    /// `quarter_turns` clockwise rotations (taken mod 4).
    pub fn rotated(&self, quarter_turns: u8) -> Shape {
        let mut s = self.clone();
        for _ in 0..(quarter_turns % 4) {
            s = s.rotated_cw();
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_normalizes_to_the_origin() {
        let s = Shape::new(&[(3, 5), (4, 5)]);
        assert_eq!(s.cells(), &[(0, 0), (1, 0)]);
    }

    #[test]
    fn width_and_height_describe_the_bounding_box() {
        let ell = Shape::new(&[(0, 0), (0, 1), (0, 2), (1, 2)]);
        assert_eq!(ell.width(), 2);
        assert_eq!(ell.height(), 3);
        assert_eq!(ell.area(), 4);
    }

    #[test]
    fn a_quarter_turn_makes_a_horizontal_domino_vertical() {
        let flat = Shape::new(&[(0, 0), (1, 0)]);
        assert_eq!(flat.rotated_cw().cells(), &[(0, 0), (0, 1)]);
    }

    #[test]
    fn four_quarter_turns_return_the_original_shape() {
        let ell = Shape::new(&[(0, 0), (0, 1), (0, 2), (1, 2)]);
        assert_eq!(ell.rotated(4), ell);
        assert_ne!(ell.rotated(1), ell, "one turn must actually change it");
    }

    #[test]
    fn rotation_preserves_area() {
        let ell = Shape::new(&[(0, 0), (0, 1), (0, 2), (1, 2)]);
        for turns in 0..4 {
            assert_eq!(ell.rotated(turns).area(), 4);
        }
    }
}
