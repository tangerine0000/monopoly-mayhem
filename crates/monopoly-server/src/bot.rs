use monopoly_core::board::{get_board, TileKind};
use monopoly_core::money::Money;
use monopoly_core::state::{GameState, TurnPhase};
use monopoly_proto::{ClientMessage, PlayerId};

pub struct XeOmBot;

impl XeOmBot {
    pub fn decide_action(state: &GameState, player_id: PlayerId) -> Option<ClientMessage> {
        let player = state.find_player(player_id)?;
        if player.is_bankrupt {
            return None;
        }

        match state.turn_phase {
            TurnPhase::WaitingForRoll => {
                if player.in_jail {
                    if player.get_out_of_jail_cards > 0 {
                        Some(ClientMessage::UseJailCard)
                    } else if player.cash > Money::from_vnd(5_000_000) {
                        Some(ClientMessage::PayJailBail)
                    } else {
                        Some(ClientMessage::RollDice)
                    }
                } else {
                    Some(ClientMessage::RollDice)
                }
            }
            TurnPhase::TileActionPending => {
                let pos = player.position as usize;
                let board = get_board();
                let tile = &board[pos];
                let cost = match &tile.kind {
                    TileKind::Property(p) => p.cost,
                    TileKind::Station(s) => s.cost,
                    TileKind::Utility(u) => u.cost,
                    _ => Money::ZERO,
                };

                if cost > Money::ZERO && player.cash >= Money(cost.0 * 2) {
                    Some(ClientMessage::BuyProperty)
                } else {
                    Some(ClientMessage::DeclineProperty)
                }
            }
            TurnPhase::Auction => {
                let auction = state.auction.as_ref()?;
                let board = get_board();
                let tile = &board[auction.tile_index as usize];
                let cost = match &tile.kind {
                    TileKind::Property(p) => p.cost,
                    TileKind::Station(s) => s.cost,
                    TileKind::Utility(u) => u.cost,
                    _ => Money::ZERO,
                };

                let half_cost = Money(cost.0 / 2);
                let next_bid = auction.current_bid + Money::from_vnd(50_000);

                if auction.current_bid < half_cost
                    && player.cash >= Money(auction.current_bid.0 * 3)
                    && player.cash >= next_bid
                {
                    Some(ClientMessage::PlaceBid { amount: next_bid })
                } else {
                    Some(ClientMessage::PassAuction)
                }
            }
            TurnPhase::TurnEnded => Some(ClientMessage::EndTurn),
        }
    }
}
