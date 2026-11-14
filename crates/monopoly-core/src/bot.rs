use crate::board::{BoardCatalog, TileKind};
use crate::state::GameState;
use monopoly_proto::{ClientMessage, Money, PlayerId, PublicGameView, PublicTurnPhase};

pub struct BotPolicy;

impl BotPolicy {
    pub fn decide_action(view: &PublicGameView, player_id: PlayerId) -> Option<ClientMessage> {
        let player = view.find_player(player_id)?;
        if player.is_bankrupt {
            return None;
        }

        match view.turn_phase {
            PublicTurnPhase::WaitingForRoll => {
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
            PublicTurnPhase::TileActionPending => {
                let tile = BoardCatalog::tile(player.position);
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
            PublicTurnPhase::Auction => {
                let auction = view.auction.as_ref()?;
                let tile = BoardCatalog::tile(auction.tile_index);
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
            PublicTurnPhase::TurnEnded => Some(ClientMessage::EndTurn),
        }
    }

    pub fn decide_action_state(state: &GameState, player_id: PlayerId) -> Option<ClientMessage> {
        Self::decide_action(&state.to_public_view(), player_id)
    }
}
