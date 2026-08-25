use std::collections::{HashMap, HashSet, VecDeque};

use crate::piece::{PieceId, PieceRegistry, SlotKind};

pub const SLOT_W: u8 = 6;
/// How tall a grid starts. It is no longer how tall a grid *is*: a run can be
/// given more rows, and `Slot::rows` is the figure that decides anything.
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
    /// How many rows this grid has. Starts at `SLOT_H` and can be grown.
    rows: u8,
    cells: Vec<Option<PieceId>>,
    /// The enchantment layer, one cell for one cell with `cells`.
    ///
    /// Two layers rather than one cell holding two pieces, because everything
    /// that reads a grid - grouping, adjacency, empty-cell counting, the
    /// diagonal relation - wants the answer "what gear is here", and an
    /// enchantment is not gear. Keeping it in its own layer means `get` still
    /// means what it always meant and none of those had to learn about the
    /// layer underneath. It is also how "an enchantment never joins an item"
    /// enforces itself: `groups` walks `cells`, so an enchantment is not there
    /// to be found - and in particular two items cannot be merged into one by
    /// laying an enchantment under both.
    ///
    /// One layer deep, and enchantments never overlap each other, so one slot
    /// per cell is enough.
    enchant: Vec<Option<PieceId>>,
}

impl Slot {
    pub fn new(kind: SlotKind) -> Self {
        Self::with_rows(kind, SLOT_H)
    }

    pub fn with_rows(kind: SlotKind, rows: u8) -> Self {
        let n = SLOT_W as usize * rows as usize;
        Self { kind, rows, cells: vec![None; n], enchant: vec![None; n] }
    }

    pub fn rows(&self) -> u8 {
        self.rows
    }

    /// Add rows to the bottom, keeping everything where it is.
    ///
    /// Cells are stored row-major, so new rows are new indices on the end and
    /// every existing one keeps its address. Growing a board a player has
    /// already filled must not move a single piece, and this is why it cannot.
    pub fn grow(&mut self, by: u8) {
        self.rows += by;
        let n = SLOT_W as usize * self.rows as usize;
        self.cells.resize(n, None);
        self.enchant.resize(n, None);
    }

    #[inline]
    fn idx(&self, x: u8, y: u8) -> usize {
        debug_assert!(x < SLOT_W && y < self.rows);
        y as usize * SLOT_W as usize + x as usize
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < SLOT_W as i32 && y < self.rows as i32
    }

    /// What gear is in this cell. An enchantment is not gear: it is under the
    /// grid, not in it, and `enchant_at` is how you ask about that.
    pub fn get(&self, x: u8, y: u8) -> Option<PieceId> {
        self.cells[self.idx(x, y)]
    }

    /// What enchantment lies under this cell, whether or not gear covers it.
    pub fn enchant_at(&self, x: u8, y: u8) -> Option<PieceId> {
        self.enchant[self.idx(x, y)]
    }

    /// Which layer a piece belongs in.
    fn layer_of(reg: &PieceRegistry, id: PieceId) -> bool {
        reg.def(id).kind.is_enchantment()
    }

    /// Every cell `id` occupies on the enchantment layer.
    pub fn enchant_cells(&self, id: PieceId) -> Vec<(u8, u8)> {
        let mut out = Vec::new();
        for y in 0..self.rows {
            for x in 0..SLOT_W {
                if self.enchant_at(x, y) == Some(id) {
                    out.push((x, y));
                }
            }
        }
        out
    }

    /// Is this enchantment live?
    ///
    /// An enchantment pays nothing unless nothing else on its own layer is
    /// touching it. The edge of the board counts as clear - an enchantment
    /// cannot be laid out of bounds, so a rule that punished the rim would
    /// punish the only cells with nowhere to crowd from.
    ///
    /// Note the two layers pull in opposite directions, which is the whole of
    /// the mechanic: enchantments have to be spread out to be live, and gear
    /// has to be packed tight on top of one to bond with it.
    pub fn enchant_is_live(&self, id: PieceId) -> bool {
        let mine = self.enchant_cells(id);
        if mine.is_empty() {
            return false;
        }
        for &(x, y) in &mine {
            for (dx, dy) in [(0i32, -1i32), (0, 1), (-1, 0), (1, 0)] {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if !self.in_bounds(nx, ny) {
                    continue;
                }
                match self.enchant_at(nx as u8, ny as u8) {
                    Some(other) if other != id => return false,
                    _ => {}
                }
            }
        }
        true
    }

    /// Is every cell of this enchantment covered by gear?
    ///
    /// Half of bonding. The other half - that all of it is *one* item - needs
    /// the groups, which live a layer up in `Loadout`.
    pub fn enchant_is_buried(&self, id: PieceId) -> bool {
        let mine = self.enchant_cells(id);
        !mine.is_empty() && mine.iter().all(|&(x, y)| self.get(x, y).is_some())
    }

    /// Distinct pieces of gear sitting on top of `id`, which must be an
    /// enchantment. Empty for anything else, and empty for one nobody has
    /// covered.
    pub fn covering(&self, id: PieceId) -> Vec<PieceId> {
        let mut out: Vec<PieceId> = Vec::new();
        for y in 0..self.rows {
            for x in 0..SLOT_W {
                if self.enchant_at(x, y) != Some(id) {
                    continue;
                }
                if let Some(on_top) = self.get(x, y) {
                    if !out.contains(&on_top) {
                        out.push(on_top);
                    }
                }
            }
        }
        out
    }

    pub fn is_empty(&self) -> bool {
        self.cells.iter().chain(self.enchant.iter()).all(|c| c.is_none())
    }

    /// Every piece currently in this slot, in a stable (row-major) order.
    pub fn pieces(&self) -> Vec<PieceId> {
        let mut seen = Vec::new();
        for cell in self.cells.iter().chain(self.enchant.iter()) {
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
        for y in 0..self.rows {
            for x in 0..SLOT_W {
                if self.get(x, y) == Some(id) || self.enchant_at(x, y) == Some(id) {
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
        self.cells.iter().chain(self.enchant.iter()).any(|c| *c == Some(id))
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
        // An enchantment collides with an enchantment and gear collides with
        // gear; the two layers do not see each other, which is the whole of
        // "an enchantment may be covered". Bounds are checked against the
        // grid's *current* rows,
        // because a board that has been granted extra rows is that tall now.
        let underlay = Self::layer_of(reg, id);
        for &(dx, dy) in reg.shape(id).cells() {
            let (nx, ny) = (ax as i32 + dx as i32, ay as i32 + dy as i32);
            if !self.in_bounds(nx, ny) {
                return Err(PlaceError::OutOfBounds);
            }
            let (x, y) = (nx as u8, ny as u8);
            let here = if underlay { self.enchant_at(x, y) } else { self.get(x, y) };
            match here {
                None => {}
                Some(other) if other == id => {}
                Some(_) => return Err(PlaceError::Occupied),
            }
        }
        Ok(())
    }

    /// Write `id` into every cell of its shape, in whichever layer it belongs
    /// to. Check `can_place` first.
    pub fn place(&mut self, reg: &PieceRegistry, id: PieceId, ax: u8, ay: u8) {
        let underlay = Self::layer_of(reg, id);
        for &(dx, dy) in reg.shape(id).cells() {
            let (nx, ny) = (ax as i32 + dx as i32, ay as i32 + dy as i32);
            if self.in_bounds(nx, ny) {
                let i = self.idx(nx as u8, ny as u8);
                if underlay {
                    self.enchant[i] = Some(id);
                } else {
                    self.cells[i] = Some(id);
                }
            }
        }
    }

    /// Clear every cell holding `id`.
    pub fn remove(&mut self, id: PieceId) {
        for cell in self.cells.iter_mut().chain(self.enchant.iter_mut()) {
            if *cell == Some(id) {
                *cell = None;
            }
        }
    }

    pub fn clear(&mut self) {
        for cell in self.cells.iter_mut().chain(self.enchant.iter_mut()) {
            *cell = None;
        }
    }

    /// Every anchor at which `id` currently fits. The GUI highlights whatever
    /// this returns — it must never work out fit for itself.
    pub fn legal_anchors(&self, reg: &PieceRegistry, id: PieceId) -> Vec<(u8, u8)> {
        let mut out = Vec::new();
        for y in 0..self.rows {
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
        for y in 0..self.rows {
            for x in 0..SLOT_W {
                if self.get(x, y) == Some(id) || self.enchant_at(x, y) == Some(id) {
                    out.push((x, y));
                }
            }
        }
        out
    }

    /// The four orthogonal neighbours of `(x, y)` that lie inside the grid.
    fn orthogonal(&self, x: u8, y: u8) -> Vec<(u8, u8)> {
        [(0i32, -1i32), (0, 1), (-1, 0), (1, 0)]
            .iter()
            .filter_map(|&(dx, dy)| {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                self.in_bounds(nx, ny).then_some((nx as u8, ny as u8))
            })
            .collect()
    }

    /// Distinct pieces orthogonally touching `id` (never `id` itself).
    pub fn neighbors_of(&self, id: PieceId) -> Vec<PieceId> {
        let mut out: Vec<PieceId> = Vec::new();
        for (x, y) in self.cells_of(id) {
            for (nx, ny) in self.orthogonal(x, y) {
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
            for (nx, ny) in self.orthogonal(x, y) {
                if self.get(nx, ny).is_none() {
                    seen.insert((nx, ny));
                }
            }
        }
        seen.len()
    }

    /// In-bounds empty cells orthogonally touching any part of a set of
    /// pieces, counted once each.
    ///
    /// The item-wide version of `empty_neighbor_cells`. Per-piece would be the
    /// stricter reading, but a component packed inside its own item touches
    /// nothing empty however airy the build is - which would make anything
    /// keyed on it dead the moment the item was worth building. This asks
    /// whether the *item* was given room.
    pub fn open_cells_around(&self, pieces: &[PieceId]) -> usize {
        let own: HashSet<(u8, u8)> = pieces.iter().flat_map(|&p| self.cells_of(p)).collect();
        let mut seen: HashSet<(u8, u8)> = HashSet::new();
        for &(x, y) in &own {
            for (nx, ny) in self.orthogonal(x, y) {
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

        for y in 0..self.rows {
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
                    for (nx, ny) in self.orthogonal(cx, cy) {
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
            .any(|(x, y)| self.orthogonal(x, y).iter().any(|c| b_cells.contains(c)))
    }

    /// Cells sharing a corner with `(x, y)` and no edge. In bounds only, so a
    /// piece against a wall simply has fewer of them.
    fn corners(&self, x: u8, y: u8) -> Vec<(u8, u8)> {
        [(-1i32, -1i32), (1, -1), (-1, 1), (1, 1)]
            .iter()
            .filter_map(|&(dx, dy)| {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                self.in_bounds(nx, ny).then_some((nx as u8, ny as u8))
            })
            .collect()
    }

    /// Do these two sets meet at a corner and nowhere along an edge?
    ///
    /// The two relations are deliberately exclusive. "Diagonal" is meant to
    /// name the pair that is *near but not touching* - an item packed against
    /// three neighbours has spent its sides, and the whole point of the
    /// relation is that it reaches past them. A pair that shares an edge
    /// somewhere is adjacent, whatever their other corners do, so it is
    /// answered by `sets_touch` and not by this.
    ///
    /// It is a relation between the two sets and not between cells: a pair that
    /// meets at four corners is diagonal once.
    pub fn sets_touch_diagonally(&self, a: &[PieceId], b: &[PieceId]) -> bool {
        if self.sets_touch(a, b) {
            return false;
        }
        let b_cells: HashSet<(u8, u8)> = b.iter().flat_map(|&p| self.cells_of(p)).collect();
        a.iter()
            .flat_map(|&p| self.cells_of(p))
            .any(|(x, y)| self.corners(x, y).iter().any(|c| b_cells.contains(c)))
    }
}
