use std::collections::{HashMap, HashSet, VecDeque};

use crate::piece::{PieceId, PieceRegistry, SlotKind};

pub const SLOT_W: u8 = 6;
pub const SLOT_H: u8 = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaceError {
    /// The piece's slot type doesn't match this slot.
    WrongSlot,
    /// Part of the shape would land outside the 6x8 grid.
    OutOfBounds,
    /// Part of the shape would land on another piece.
    Occupied,
}

impl std::fmt::Display for PlaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlaceError::WrongSlot => write!(f, "that piece doesn't belong in this slot"),
            PlaceError::OutOfBounds => write!(f, "doesn't fit - hangs off the edge"),
            PlaceError::Occupied => write!(f, "doesn't fit - something's in the way"),
        }
    }
}

/// One 6x8 equipment grid. Cells hold piece ids, so a multi-cell piece is the
/// same id repeated; the piece's data lives in the `PieceRegistry`.
#[derive(Clone, Debug)]
pub struct Slot {
    pub kind: SlotKind,
    cells: Vec<Option<PieceId>>,
}

impl Slot {
    pub fn new(kind: SlotKind) -> Self {
        Self { kind, cells: vec![None; SLOT_W as usize * SLOT_H as usize] }
    }

    #[inline]
    fn idx(x: u8, y: u8) -> usize {
        debug_assert!(x < SLOT_W && y < SLOT_H);
        y as usize * SLOT_W as usize + x as usize
    }

    pub fn in_bounds(x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < SLOT_W as i32 && y < SLOT_H as i32
    }

    pub fn get(&self, x: u8, y: u8) -> Option<PieceId> {
        self.cells[Self::idx(x, y)]
    }

    pub fn is_empty(&self) -> bool {
        self.cells.iter().all(|c| c.is_none())
    }

    /// Every piece currently in this slot, in a stable (row-major) order.
    pub fn pieces(&self) -> Vec<PieceId> {
        let mut seen = Vec::new();
        for cell in &self.cells {
            if let Some(id) = cell {
                if !seen.contains(id) {
                    seen.push(*id);
                }
            }
        }
        seen
    }

    /// The anchor cell of a placed piece: the minimum x and y of the cells it
    /// occupies. Shapes are normalized, so this always recovers the anchor the
    /// piece was placed at.
    pub fn anchor_of(&self, id: PieceId) -> Option<(u8, u8)> {
        let mut anchor: Option<(u8, u8)> = None;
        for y in 0..SLOT_H {
            for x in 0..SLOT_W {
                if self.get(x, y) == Some(id) {
                    anchor = Some(match anchor {
                        None => (x, y),
                        Some((ax, ay)) => (ax.min(x), ay.min(y)),
                    });
                }
            }
        }
        anchor
    }

    pub fn contains(&self, id: PieceId) -> bool {
        self.cells.iter().any(|c| *c == Some(id))
    }

    /// Would `id` fit with its anchor at `(ax, ay)`? Cells the piece itself
    /// already occupies don't count as collisions, so this also answers
    /// "can it be nudged there" for a piece already in the slot.
    pub fn can_place(
        &self,
        reg: &PieceRegistry,
        id: PieceId,
        ax: u8,
        ay: u8,
    ) -> Result<(), PlaceError> {
        // `fits` rather than an equality check: materials and plating are
        // shared between two grids each.
        if !reg.def(id).fits(self.kind) {
            return Err(PlaceError::WrongSlot);
        }
        for &(dx, dy) in reg.shape(id).cells() {
            let (nx, ny) = (ax as i32 + dx as i32, ay as i32 + dy as i32);
            if !Self::in_bounds(nx, ny) {
                return Err(PlaceError::OutOfBounds);
            }
            match self.get(nx as u8, ny as u8) {
                None => {}
                Some(other) if other == id => {}
                Some(_) => return Err(PlaceError::Occupied),
            }
        }
        Ok(())
    }

    /// Write `id` into every cell of its shape. Check `can_place` first.
    pub fn place(&mut self, reg: &PieceRegistry, id: PieceId, ax: u8, ay: u8) {
        for &(dx, dy) in reg.shape(id).cells() {
            let (nx, ny) = (ax as i32 + dx as i32, ay as i32 + dy as i32);
            if Self::in_bounds(nx, ny) {
                let i = Self::idx(nx as u8, ny as u8);
                self.cells[i] = Some(id);
            }
        }
    }

    /// Clear every cell holding `id`.
    pub fn remove(&mut self, id: PieceId) {
        for cell in &mut self.cells {
            if *cell == Some(id) {
                *cell = None;
            }
        }
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = None;
        }
    }

    /// Every anchor at which `id` currently fits. The GUI highlights whatever
    /// this returns — it must never work out fit for itself.
    pub fn legal_anchors(&self, reg: &PieceRegistry, id: PieceId) -> Vec<(u8, u8)> {
        let mut out = Vec::new();
        for y in 0..SLOT_H {
            for x in 0..SLOT_W {
                if self.can_place(reg, id, x, y).is_ok() {
                    out.push((x, y));
                }
            }
        }
        out
    }

    /// Every cell `id` occupies.
    pub fn cells_of(&self, id: PieceId) -> Vec<(u8, u8)> {
        let mut out = Vec::new();
        for y in 0..SLOT_H {
            for x in 0..SLOT_W {
                if self.get(x, y) == Some(id) {
                    out.push((x, y));
                }
            }
        }
        out
    }

    /// The four orthogonal neighbours of `(x, y)` that lie inside the grid.
    fn orthogonal(x: u8, y: u8) -> Vec<(u8, u8)> {
        [(0i32, -1i32), (0, 1), (-1, 0), (1, 0)]
            .iter()
            .filter_map(|&(dx, dy)| {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                Self::in_bounds(nx, ny).then_some((nx as u8, ny as u8))
            })
            .collect()
    }

    /// Distinct pieces orthogonally touching `id` (never `id` itself).
    pub fn neighbors_of(&self, id: PieceId) -> Vec<PieceId> {
        let mut out: Vec<PieceId> = Vec::new();
        for (x, y) in self.cells_of(id) {
            for (nx, ny) in Self::orthogonal(x, y) {
                if let Some(other) = self.get(nx, ny) {
                    if other != id && !out.contains(&other) {
                        out.push(other);
                    }
                }
            }
        }
        out
    }

    /// In-bounds empty cells orthogonally touching `id`'s footprint, counted
    /// once each. Cells beyond the grid edge do not count, so a piece out in
    /// open space is worth more than one shoved into a corner.
    pub fn empty_neighbor_cells(&self, id: PieceId) -> usize {
        let mut seen: HashSet<(u8, u8)> = HashSet::new();
        for (x, y) in self.cells_of(id) {
            for (nx, ny) in Self::orthogonal(x, y) {
                if self.get(nx, ny).is_none() {
                    seen.insert((nx, ny));
                }
            }
        }
        seen.len()
    }

    /// The slot's pieces partitioned into orthogonally-connected groups. Each
    /// group is a candidate item: one slot can hold as many finished items as
    /// the player can fit, so long as they don't touch each other.
    ///
    /// A piece is atomic. Most shapes are one connected blob anyway, but a few
    /// are not - the Hollow Sphere is a ring of four cells that touch only at
    /// the corners - and cell-by-cell flooding would hand back the same id as
    /// four separate items. Reaching any cell of a piece therefore reaches all
    /// of them.
    ///
    /// Groups come back ordered by their topmost-then-leftmost cell, so the
    /// UI can label them stably.
    pub fn groups(&self) -> Vec<Vec<PieceId>> {
        let mut visited: HashSet<(u8, u8)> = HashSet::new();
        let mut groups = Vec::new();

        for y in 0..SLOT_H {
            for x in 0..SLOT_W {
                if self.get(x, y).is_none() || visited.contains(&(x, y)) {
                    continue;
                }
                // Flood-fill this component, collecting the pieces it touches.
                let mut members: Vec<PieceId> = Vec::new();
                let mut queue = VecDeque::new();
                visited.insert((x, y));
                queue.push_back((x, y));
                while let Some((cx, cy)) = queue.pop_front() {
                    if let Some(id) = self.get(cx, cy) {
                        if !members.contains(&id) {
                            members.push(id);
                            // The rest of this piece comes along, connected or
                            // not, so a hollow shape stays one thing.
                            for (ox, oy) in self.cells_of(id) {
                                if visited.insert((ox, oy)) {
                                    queue.push_back((ox, oy));
                                }
                            }
                        }
                    }
                    for (nx, ny) in Self::orthogonal(cx, cy) {
                        if self.get(nx, ny).is_some() && visited.insert((nx, ny)) {
                            queue.push_back((nx, ny));
                        }
                    }
                }
                groups.push(members);
            }
        }
        groups
    }

    /// How many of each `PieceKind` appear among `pieces`.
    pub fn kind_counts(
        reg: &PieceRegistry,
        pieces: &[PieceId],
    ) -> HashMap<crate::piece::PieceKind, usize> {
        let mut counts = HashMap::new();
        for &id in pieces {
            *counts.entry(reg.def(id).kind).or_insert(0) += 1;
        }
        counts
    }

    /// The slot's pieces split into **items**, one per core piece.
    ///
    /// Every recipe names exactly one component it needs exactly one of — the
    /// handle, frame, base or material. That piece is the item's core. Other
    /// pieces join whichever core they are closest to through the touching
    /// pieces, so two finished items can sit flush against each other and stay
    /// separate. A connected blob with no core at all is one unfinished item.
    ///
    /// Deterministic: cores are seeded in row-major order and ties in the
    /// multi-source search go to the earlier core.
    pub fn items(&self, reg: &PieceRegistry) -> Vec<Vec<PieceId>> {
        self.items_with_locks(reg, &[])
    }

    /// The same, except that any set in `locked` is taken out first and kept
    /// exactly as it is.
    ///
    /// A locked item stops negotiating: nothing else can join it, and it
    /// cannot lose a piece to a neighbour. That is the point of locking one -
    /// you have decided what it is, and packing something beside it should no
    /// longer be able to change its mind.
    pub fn items_with_locks(
        &self,
        reg: &PieceRegistry,
        locked: &[crate::loadout::LockedItem],
    ) -> Vec<Vec<PieceId>> {
        let mut out = Vec::new();
        let mut spoken_for: Vec<PieceId> = Vec::new();
        for set in locked {
            let set = &set.pieces;
            let here: Vec<PieceId> =
                set.iter().copied().filter(|&p| self.contains(p)).collect();
            if here.len() == set.len() && !here.is_empty() {
                spoken_for.extend(here.iter().copied());
                out.push(here);
            }
        }

        for group in self.groups() {
            let group: Vec<PieceId> =
                group.into_iter().filter(|p| !spoken_for.contains(p)).collect();
            if group.is_empty() {
                continue;
            }
            let cores: Vec<PieceId> = group
                .iter()
                .copied()
                .filter(|&p| reg.def(p).kind.is_core())
                .collect();

            // No core, or exactly one: the blob is a single item either way.
            if cores.len() <= 1 {
                out.push(group);
                continue;
            }

            // Several cores in one blob: hand each remaining piece to its
            // nearest core, breadth-first through the piece adjacency graph.
            let mut owner: HashMap<PieceId, PieceId> = HashMap::new();
            let mut queue: VecDeque<PieceId> = VecDeque::new();
            for &c in &cores {
                owner.insert(c, c);
                queue.push_back(c);
            }
            while let Some(p) = queue.pop_front() {
                let holder = owner[&p];
                for q in self.neighbors_of(p) {
                    if !group.contains(&q) || owner.contains_key(&q) {
                        continue;
                    }
                    owner.insert(q, holder);
                    queue.push_back(q);
                }
            }

            // Emit one item per core, keeping the group's original ordering.
            for &c in &cores {
                let members: Vec<PieceId> = group
                    .iter()
                    .copied()
                    .filter(|p| owner.get(p) == Some(&c))
                    .collect();
                if !members.is_empty() {
                    out.push(members);
                }
            }
        }
        out
    }

    /// The topmost and bottommost rows a set of pieces occupies. Used for
    /// cross-slot alignment, where "lined up" means sharing rows.
    pub fn row_span(&self, pieces: &[PieceId]) -> Option<(u8, u8)> {
        let mut span: Option<(u8, u8)> = None;
        for &p in pieces {
            for (_, y) in self.cells_of(p) {
                span = Some(match span {
                    None => (y, y),
                    Some((lo, hi)) => (lo.min(y), hi.max(y)),
                });
            }
        }
        span
    }

    /// Are these pieces one orthogonally connected blob? An item whose parts
    /// are not joined is not an item.
    pub fn connected(&self, pieces: &[PieceId]) -> bool {
        let Some(&first) = pieces.first() else { return true };
        let mut seen = vec![first];
        let mut queue = vec![first];
        while let Some(p) = queue.pop() {
            for q in self.neighbors_of(p) {
                if pieces.contains(&q) && !seen.contains(&q) {
                    seen.push(q);
                    queue.push(q);
                }
            }
        }
        seen.len() == pieces.len()
    }

    /// Do these two sets of pieces touch? Used for item-to-item adjacency,
    /// which is now possible because touching no longer merges items.
    pub fn sets_touch(&self, a: &[PieceId], b: &[PieceId]) -> bool {
        let b_cells: HashSet<(u8, u8)> =
            b.iter().flat_map(|&p| self.cells_of(p)).collect();
        a.iter()
            .flat_map(|&p| self.cells_of(p))
            .any(|(x, y)| Self::orthogonal(x, y).iter().any(|c| b_cells.contains(c)))
    }
}
