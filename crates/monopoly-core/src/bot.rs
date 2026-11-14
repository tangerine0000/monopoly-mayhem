//! Bot "Xe Ôm" dùng chung cho server và chế độ chơi trên cùng máy.
//!
//! Nguyên tắc an toàn: bot chỉ trả về hành động khi chính nó đang phải hành động
//! (đang tới lượt, đang nợ, đang trong phiên đấu giá mà không dẫn đầu, hoặc đang
//! nhận đề nghị giao dịch). Mọi trường hợp khác trả `None`, nên vòng lặp gọi bot
//! luôn dừng.

use crate::board::{get_board, get_district_tiles, TileKind};
use crate::money::Money;
use crate::state::{GameState, TurnPhase};
use monopoly_proto::{ClientMessage, PlayerId};

/// Tiền mặt bot luôn giữ lại khi xây nhà.
const BUILD_RESERVE_VND: i64 = 3_000_000;

fn tile_cost(idx: u8) -> Money {
    match &get_board()[idx as usize].kind {
        TileKind::Property(p) => p.cost,
        TileKind::Station(s) => s.cost,
        TileKind::Utility(u) => u.cost,
        _ => Money::ZERO,
    }
}

/// Giá trị bot gán cho một gói đất + tiền.
fn bundle_value(state: &GameState, tiles: &[u8], cash: Money) -> i64 {
    tiles
        .iter()
        .map(|&t| {
            let c = tile_cost(t).0;
            if state.properties[t as usize].is_mortgaged {
                c / 2
            } else {
                c
            }
        })
        .sum::<i64>()
        + cash.0
}

/// Sau giao dịch, `player` có đủ bộ một quận mà trước đó chưa có không?
fn completes_district(state: &GameState, player: PlayerId, gained: &[u8]) -> bool {
    let board = get_board();
    gained.iter().any(|&t| {
        if let TileKind::Property(p) = &board[t as usize].kind {
            get_district_tiles(p.district).iter().all(|&d| {
                d == t || state.properties[d as usize].owner == Some(player) || gained.contains(&d)
            })
        } else {
            false
        }
    })
}

/// Bước gom tiền tiếp theo khi đang nợ: bán nhà trước, thế chấp sau (ô rẻ trước).
fn raise_funds_step(state: &GameState, me: PlayerId) -> Option<ClientMessage> {
    let board = get_board();
    // Bán nhà ở ô có nhiều nhà nhất trong quận (tuân theo luật bán đều)
    let mut best: Option<(u8, u8)> = None;
    for (idx, prop) in state.properties.iter().enumerate() {
        if prop.owner == Some(me) && prop.house_count > 0 {
            if let TileKind::Property(p) = &board[idx].kind {
                let max = get_district_tiles(p.district)
                    .iter()
                    .map(|&d| state.properties[d as usize].house_count)
                    .max()
                    .unwrap_or(0);
                if prop.house_count == max && best.map(|b| prop.house_count > b.1).unwrap_or(true) {
                    best = Some((idx as u8, prop.house_count));
                }
            }
        }
    }
    if let Some((tile, _)) = best {
        return Some(ClientMessage::SellHouse { tile_index: tile });
    }
    // Thế chấp ô rẻ nhất không có nhà trong quận
    let mut candidates: Vec<u8> = state
        .properties
        .iter()
        .enumerate()
        .filter(|(idx, prop)| {
            prop.owner == Some(me)
                && !prop.is_mortgaged
                && match &board[*idx].kind {
                    TileKind::Property(p) => get_district_tiles(p.district)
                        .iter()
                        .all(|&d| state.properties[d as usize].house_count == 0),
                    TileKind::Station(_) | TileKind::Utility(_) => true,
                    _ => false,
                }
        })
        .map(|(idx, _)| idx as u8)
        .collect();
    candidates.sort_by_key(|&t| tile_cost(t));
    candidates
        .first()
        .map(|&t| ClientMessage::MortgageProperty { tile_index: t })
}

/// Ô nên xây nhà tiếp theo (đủ bộ, không thế chấp, xây đều, còn đủ tiền dự phòng).
fn build_step(state: &GameState, me: PlayerId) -> Option<ClientMessage> {
    let board = get_board();
    let cash = state.find_player(me)?.cash;
    for (idx, tile) in board.iter().enumerate() {
        let TileKind::Property(p) = &tile.kind else {
            continue;
        };
        if state.properties[idx].owner != Some(me) || !state.owns_full_district(me, p.district) {
            continue;
        }
        let tiles = get_district_tiles(p.district);
        let min = tiles
            .iter()
            .map(|&d| state.properties[d as usize].house_count)
            .min()
            .unwrap_or(0);
        let here = state.properties[idx].house_count;
        if here == min
            && here < 5
            && cash.0 - p.house_cost.0 >= Money::from_vnd(BUILD_RESERVE_VND).0
        {
            return Some(ClientMessage::BuildHouse {
                tile_index: idx as u8,
            });
        }
    }
    None
}

pub fn decide_action(state: &GameState, me: PlayerId) -> Option<ClientMessage> {
    if !state.is_game_started || state.is_game_over {
        return None;
    }
    let player = state.find_player(me)?;
    if player.is_bankrupt {
        return None;
    }

    // 1. Đề nghị giao dịch gửi cho mình
    if let Some(t) = state.trade.as_ref().filter(|t| t.to == me) {
        let receive = bundle_value(state, &t.give_tiles, t.give_cash);
        let give = bundle_value(state, &t.take_tiles, t.take_cash);
        let helps_rival = completes_district(state, t.from, &t.take_tiles);
        let threshold = if helps_rival { give * 2 } else { give * 5 / 4 };
        return Some(ClientMessage::RespondTrade {
            accept: receive > 0 && receive >= threshold,
        });
    }

    // 2. Đấu giá: bất kỳ ai còn trong phiên, trừ người đang dẫn
    if state.turn_phase == TurnPhase::Auction {
        let a = state.auction.as_ref()?;
        if !a.active_bidders.contains(&me) || a.highest_bidder == Some(me) {
            return None;
        }
        let cap = Money(tile_cost(a.tile_index).0 * 3 / 4);
        let next = a.current_bid + Money::from_vnd(100_000);
        return if next <= cap && player.cash.0 >= next.0 * 2 {
            Some(ClientMessage::PlaceBid { amount: next })
        } else {
            Some(ClientMessage::PassAuction)
        };
    }

    // 3. Các pha còn lại chỉ dành cho người đang tới lượt
    if state.current_player().map(|p| p.id) != Some(me) {
        return None;
    }

    match state.turn_phase {
        TurnPhase::Debt => {
            let debt = state.debt.as_ref()?;
            if player.cash >= debt.amount {
                Some(ClientMessage::PayDebt)
            } else {
                raise_funds_step(state, me).or(Some(ClientMessage::DeclareBankruptcy))
            }
        }
        TurnPhase::WaitingForRoll => {
            if player.in_jail {
                if player.get_out_of_jail_cards > 0 {
                    return Some(ClientMessage::UseJailCard);
                }
                if player.cash > Money::from_vnd(5_000_000) {
                    return Some(ClientMessage::PayJailBail);
                }
            }
            Some(ClientMessage::RollDice)
        }
        TurnPhase::TileActionPending => {
            let cost = tile_cost(player.position);
            if cost > Money::ZERO && player.cash.0 >= cost.0 * 3 / 2 {
                Some(ClientMessage::BuyProperty)
            } else {
                Some(ClientMessage::DeclineProperty)
            }
        }
        TurnPhase::TurnEnded => build_step(state, me).or(Some(ClientMessage::EndTurn)),
        TurnPhase::Auction => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::apply_action;
    use crate::state::PlayerState;
    use rand::SeedableRng;
    use uuid::Uuid;

    fn game(n: usize) -> GameState {
        let mut s = GameState::new();
        for i in 0..n {
            let id = PlayerId(Uuid::from_u128(i as u128 + 1));
            let mut p = PlayerState::new(id, format!("Bot {i}"), i as u8, None);
            p.is_bot_controlled = true;
            s.players.push(p);
            s.turn_order.push(id);
        }
        s.is_game_started = true;
        s
    }

    /// Cho 4 bot tự chơi nhiều ván: mọi hành động bot chọn phải hợp lệ, và ván
    /// luôn tiến triển (không kẹt ở trạng thái mà không ai hành động được).
    #[test]
    fn bots_play_full_games_without_stalling_or_invalid_actions() {
        for seed in 0..12u64 {
            let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
            let mut s = game(4);
            for step in 0..6000 {
                if s.is_game_over {
                    break;
                }
                let actor = s
                    .players
                    .iter()
                    .map(|p| p.id)
                    .find(|&id| decide_action(&s, id).is_some())
                    .unwrap_or_else(|| {
                        panic!(
                            "seed {seed} bước {step}: không ai hành động, pha {:?}",
                            s.turn_phase
                        )
                    });
                let action = decide_action(&s, actor).unwrap();
                let (next, _) = apply_action(&s, actor, &action, &mut rng).unwrap_or_else(|e| {
                    panic!("seed {seed} bước {step}: {action:?} bị từ chối: {e}")
                });
                s = next;
            }
        }
    }

    #[test]
    fn bot_never_acts_when_not_its_responsibility() {
        let s = game(3);
        let second = s.players[1].id;
        assert_eq!(decide_action(&s, second), None);
    }
}
