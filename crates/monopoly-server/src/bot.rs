//! Bot "Xe Ôm" của server: dùng chung logic với client trong `monopoly_core::bot`.
use monopoly_core::state::GameState;
use monopoly_proto::{ClientMessage, PlayerId};

pub struct XeOmBot;

impl XeOmBot {
    pub fn decide_action(state: &GameState, player_id: PlayerId) -> Option<ClientMessage> {
        monopoly_core::bot::decide_action(state, player_id)
    }
}
