use leptos::*;
use monopoly_proto::{
    PlayerId, PublicGameConfig, PublicGameView, PublicPropertyView, PublicTurnPhase,
};

pub const MAX_LOG_ENTRIES: usize = 150;

pub fn empty_public_view() -> PublicGameView {
    PublicGameView {
        config: PublicGameConfig {
            enable_auction: true,
            max_players: 4,
            turn_time_limit_seconds: 120,
        },
        players: Vec::new(),
        turn_order: Vec::new(),
        current_player_index: 0,
        properties: vec![PublicPropertyView::default(); 40],
        turn_phase: PublicTurnPhase::WaitingForRoll,
        doubles_count: 0,
        last_dice: (1, 1),
        auction: None,
        is_game_started: false,
        is_game_over: false,
        winner: None,
    }
}

#[derive(Clone, Copy)]
pub struct AnimationQueue {
    pub generation: RwSignal<u64>,
}

impl AnimationQueue {
    pub fn new() -> Self {
        Self {
            generation: create_rw_signal(0),
        }
    }

    /// Bumps the generation token, cancelling all previously scheduled animation timers.
    pub fn cancel_all(&self) -> u64 {
        let mut next = 0;
        self.generation.update(|g| {
            *g = g.wrapping_add(1);
            next = *g;
        });
        next
    }

    pub fn current(&self) -> u64 {
        self.generation.get_untracked()
    }

    pub fn is_current(&self, token: u64) -> bool {
        self.generation.get_untracked() == token
    }
}

impl Default for AnimationQueue {
    fn default() -> Self {
        Self::new()
    }
}

pub fn push_bounded_log(logs: WriteSignal<Vec<String>>, entry: String) {
    logs.update(|list| {
        if list.len() >= MAX_LOG_ENTRIES {
            let overflow = list.len() + 1 - MAX_LOG_ENTRIES;
            list.drain(0..overflow);
        }
        list.push(entry);
    });
}

pub fn tile_property(view: Signal<PublicGameView>, tile_idx: usize) -> PublicPropertyView {
    view.with(|v| v.properties.get(tile_idx).copied().unwrap_or_default())
}

pub fn tile_owner(view: Signal<PublicGameView>, tile_idx: usize) -> Option<PlayerId> {
    view.with(|v| v.properties.get(tile_idx).and_then(|p| p.owner))
}
