//! Reading the screen off the run.
//!
//! One rule, and `tests/view.rs` holds it: every accessor used here is one the
//! GUI also calls. The screen an agent reads is therefore the screen a person
//! reads, field by field, and the test that says so walks the GUI's source for
//! the list.

use crate::view::*;
use crate::Console;
use gearmaster_engine::piece::{PieceDef, PieceId, SlotKind};
use gearmaster_engine::run::{Phase, Run};
use gearmaster_engine::slot::SLOT_W;

fn describe_piece(def: &'static PieceDef, id: Option<PieceId>, w: u8, h: u8, price: i32) -> Piece {
    Piece {
        id,
        name: def.name.to_string(),
        slot: def.slot,
        kind: def.kind,
        role: def.kind.name_in(def.slot).to_string(),
        width: w,
        height: h,
        cells: def.cells.len() as u8,
        stats: def.base,
        when: def
            .base
            .summary_by_when()
            .into_iter()
            .map(|(w, text)| (format!("{:?}", w).to_lowercase(), text))
            .collect(),
        price,
        triggers: def.triggers.iter().map(|t| t.describe()).collect(),
        effect: def.effect.map(|e| e.describe()),
        assembly_bonus: def
            .assembly_bonus
            .map(|b| format!("{} ({})", b.label, b.stats.summary())),
    }
}

fn figures_of(run: &Run) -> Figures {
    let f = run.county_figures();
    Figures {
        flow: f.flow,
        physical_dps: f.physical_dps,
        magic_dps: f.magic_dps,
        armour_ps: f.armour_ps,
        fastest_ms: f.fastest_ms,
        curse_resist: f.curse_resist,
    }
}

/// A fight, read off its log.
pub(crate) fn fight_of(log: &gearmaster_engine::combat::CombatLog) -> Fight {
    let won = log.outcome == gearmaster_engine::combat::Outcome::Victory;
    Fight {
        outcome: log.outcome.label().to_string(),
        won,
        duration_ms: log.duration_ms,
        // A fight past thirty seconds was decided by the clock and not by the
        // board (`CLAUDE.md` §6 trap 5), and the two are counted in separate
        // columns everywhere in this mission.
        board_decided: won && log.duration_ms < gearmaster_engine::combat::SUDDEN_DEATH_MS,
        against: log.enemy().name.clone(),
        entries: log.entries.len(),
        health_left: log.player.health,
        enemy_health_left: log.enemy().health,
    }
}

impl Console {
    pub(crate) fn build_view(&self) -> View {
        let run = &self.run;

        // ---- the five grids ----------------------------------------------
        let mut grids = Vec::new();
        for kind in SlotKind::ALL {
            let slot = run.loadout.slot(kind);
            let rep = run.report(kind);
            let rows = slot.rows();
            let mut cells = Vec::with_capacity(rows as usize * SLOT_W as usize);
            for y in 0..rows {
                for x in 0..SLOT_W {
                    let piece = slot.get(x, y);
                    let item = piece.and_then(|id| {
                        rep.items.iter().position(|it| it.pieces.contains(&id))
                    });
                    cells.push(Cell { piece, item });
                }
            }
            let items = rep
                .items
                .iter()
                .map(|it| Item {
                    name: if it.assembled {
                        it.name.full.clone()
                    } else {
                        "(unfinished)".to_string()
                    },
                    assembled: it.assembled,
                    locked: it.pieces.first().is_some_and(|&p| run.is_locked_item(p)),
                    status: it.status.clone(),
                    stats: it.stats,
                    pieces: it.pieces.clone(),
                    notes: it.notes.clone(),
                })
                .collect();
            grids.push(Grid {
                slot: kind,
                rows,
                recipes: gearmaster_engine::piece::recipe_parts(kind)
                    .into_iter()
                    .map(|r| Recipe {
                        title: r.title.to_string(),
                        required: r.required,
                        optional: r.optional,
                    })
                    .collect(),
                cells,
                summary: rep.summary(),
                stats: rep.stats,
                items,
            });
        }

        // ---- the tray ----------------------------------------------------
        let tray = run
            .inventory()
            .into_iter()
            .map(|id| {
                let def = run.registry.def(id);
                let sh = run.registry.shape(id);
                describe_piece(def, Some(id), sh.width(), sh.height(), def.price)
            })
            .collect();

        // ---- the shelves -------------------------------------------------
        let mut shop = Vec::new();
        for i in 0..run.shop.stock.len() {
            let Some(def) = run.shop.def(i) else { continue };
            let price = run.price(i);
            let sh = gearmaster_engine::shape::Shape::new(def.cells);
            shop.push(Shelf {
                index: i,
                piece: describe_piece(def, None, sh.width(), sh.height(), def.price),
                price,
                pinned: run.shop.is_locked(i),
                affordable: price.is_some_and(|p| p <= run.gold),
                barter: run.payment_for(i),
            });
        }

        // ---- the road ----------------------------------------------------
        let road = run
            .road_stack()
            .iter()
            .map(|i| RoadItem { kind: i.kind().to_string(), describe: run.theme.retell(&i.describe()) })
            .collect();

        let question = run.pending_event().map(|e| Question {
            id: e.id.to_string(),
            title: run.theme.place(e.id, e.title).to_string(),
            scene: run.theme.scene(e.id, e.prose).iter().map(|l| run.theme.retell(l)).collect(),
            choices: e
                .choices
                .iter()
                .enumerate()
                .map(|(i, c)| Choice {
                    index: i,
                    label: c.label.to_string(),
                    blurb: c.blurb.to_string(),
                    open: run.choice_open(c),
                    requires: c.requires.describe(),
                    unmet: c.unmet.to_string(),
                    figure: match c.requires {
                        gearmaster_engine::event::Requirement::Figure { min, max } => {
                            Some((min, max))
                        }
                        _ => None,
                    },
                })
                .collect(),
        });

        let town = run.pending_town().map(|t| Town {
            name: t.name.to_string(),
            blurb: t.blurb.iter().map(|l| run.theme.retell(l)).collect(),
            doors: t.actions.iter().map(|a| (*a, a.blurb().to_string())).collect(),
        });

        let points = run.dungeon.filter(|_| run.at_points).map(|(d, floor)| Points {
            fork: d.floors[floor].fork.iter().map(|l| run.theme.retell(l)).collect(),
            exits: d.floors[floor]
                .exits
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    (i, e.label.to_string(), e.blurb.to_string(), run.has_cleared(d.id, e.to))
                })
                .collect(),
        });

        let dungeon = run.dungeon.map(|(d, at)| {
            let entered = run.dungeons_entered.contains(&d.id);
            DungeonMap {
                id: d.id.to_string(),
                name: d.name.to_string(),
                at,
                entered,
                floors: d
                    .floors
                    .iter()
                    .enumerate()
                    .map(|(i, f)| Floor {
                        index: i,
                        creature: (entered || run.has_cleared(d.id, i))
                            .then(|| f.creature.to_string()),
                        cleared: run.has_cleared(d.id, i),
                        exits: f.exits.iter().map(|e| (e.to, e.label.to_string())).collect(),
                    })
                    .collect(),
            }
        });

        let county = run.county_at.map(|at| {
            let c = run.county();
            let moves_left = run
                .road_stack()
                .iter()
                .find_map(|i| match i {
                    gearmaster_engine::run::Interrupt::County { moves_left, .. } => {
                        Some(*moves_left)
                    }
                    _ => None,
                })
                .unwrap_or(0);
            County {
                at,
                reference: gearmaster_engine::county::reference(at),
                here: run.theme.retell(c.at(at).kind.what()),
                moves_left,
                around: gearmaster_engine::county::Step::ALL
                    .into_iter()
                    .map(|step| {
                        let n = step.from(at).map(|to| Neighbour {
                            at: to,
                            reference: gearmaster_engine::county::reference(to),
                            what: run.theme.retell(c.at(to).kind.what()),
                            cleared: run.county_is_cleared(to),
                            sealed: c.is_sealed(to) && !run.pale_is_open(),
                            // One tile away and not before, unless the
                            // Surveyor's sheet says otherwise - which is
                            // `county_threshold_known`'s own rule, not this
                            // screen's.
                            threshold: match c.at(to).kind {
                                gearmaster_engine::county::TileKind::Feature(t)
                                    if run.county_threshold_known(to) =>
                                {
                                    Some(t.threshold())
                                }
                                _ => None,
                            },
                        });
                        (step.key().to_string(), n)
                    })
                    .collect(),
                trips_left: gearmaster_engine::run::trip_cap()
                    .saturating_sub(run.county_trips.len()),
                clock: run.events_resolved as usize,
                figures: figures_of(run),
                checklist: run
                    .pale_checklist()
                    .into_iter()
                    .map(|(r, done)| (r.describe(), done))
                    .collect(),
            }
        });

        let fountain = if run.at_fountain() {
            Some(Fountain {
                doubling: false,
                offer: run
                    .fountain_offer()
                    .iter()
                    .map(|c| (c.name.to_string(), c.blurb.to_string()))
                    .collect(),
            })
        } else if run.at_doubling_fountain() {
            Some(Fountain {
                doubling: true,
                offer: run
                    .doubling_offer()
                    .iter()
                    .map(|c| (c.name.to_string(), c.blurb.to_string()))
                    .collect(),
            })
        } else {
            None
        };

        // ---- what is coming ----------------------------------------------
        //
        // Everything the portrait card draws, which is everything: stats, the
        // items it will swing, its innate attacks. See this module's header.
        let spec = run.monster();
        let (mstats, mitems) = spec.outfit();
        let coming = Coming {
            name: spec.name.to_string(),
            rung_shown: run.rung + 1,
            stats: mstats,
            brings: mitems.iter().map(|p| (p.name.clone(), p.cooldown_ms)).collect(),
            innate: spec
                .attacks
                .iter()
                .map(|a| format!("{} every {:.1}s", a.name, a.cooldown_ms as f32 / 1000.0))
                .collect(),
            bounty: spec.bounty,
        };

        // The log while a fight is being replayed, and the console's own copy
        // of the last one after it has settled - because `back_to_loadout`
        // clears the run's.
        let last_fight = run.log.as_ref().map(fight_of).or_else(|| self.last.clone());

        View {
            rung_shown: run.rung + 1,
            gold: run.gold,
            wins: run.wins,
            losses: run.losses,
            lives_left: run.lives_left(),
            grinder: run.mode == gearmaster_engine::run::Mode::Grinder,
            fighting: run.phase == Phase::Fighting,
            over: self.over(),
            classes: run.classes.iter().map(|c| c.name.to_string()).collect(),
            stats: run.player_stats(),
            figures: figures_of(run),
            grids,
            tray,
            tray_cap: gearmaster_engine::run::INVENTORY_CAP,
            shop,
            reroll_cost: run.reroll_cost(),
            road,
            blocked: run.road_is_blocked().map(|s| s.to_string()),
            question,
            town,
            points,
            county,
            fountain,
            in_dungeon: run.dungeon.is_some(),
            dungeon,
            brawl_waiting: run.pending_brawl().is_some(),
            coming,
            last_fight,
            receipt: run.last_receipt.clone().unwrap_or_default(),
            undoable: run.undoable().map(|s| s.to_string()),
        }
    }
}
