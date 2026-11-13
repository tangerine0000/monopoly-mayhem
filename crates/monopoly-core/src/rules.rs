use crate::board::{get_board, get_district_tiles, TileKind};
use crate::cards::CardEffect;
use crate::money::Money;
use crate::state::{DebtSession, GameState, TradeOffer, TurnPhase, JAIL_BAIL_VND, PASS_GO_REWARD_VND};
use monopoly_proto::{ClientMessage, PlayerId, ServerMessage};
use rand::Rng;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum GameRuleError {
    #[error("Game is not started")]
    GameNotStarted,
    #[error("Game is already over")]
    GameOver,
    #[error("Not current player's turn: {0}")]
    NotCurrentPlayer(PlayerId),
    #[error("Invalid phase: expected {expected}, got {actual:?}")]
    InvalidPhase {
        expected: &'static str,
        actual: TurnPhase,
    },
    #[error("Player {0} is bankrupt")]
    PlayerBankrupt(PlayerId),
    #[error("Player {0} is in jail")]
    PlayerInJail(PlayerId),
    #[error("Player {0} is not in jail")]
    PlayerNotInJail(PlayerId),
    #[error("No get-out-of-jail card available")]
    NoJailCard,
    #[error("Insufficient funds: needed {needed}, available {available}")]
    InsufficientFunds { needed: Money, available: Money },
    #[error("Tile {0} is not purchasable")]
    TileNotPurchasable(u8),
    #[error("Tile {0} is already owned")]
    TileAlreadyOwned(u8),
    #[error("Player does not own tile {0}")]
    NotPropertyOwner(u8),
    #[error("Must own all properties in district to build")]
    MustOwnMonopolyToBuild,
    #[error("Cannot build: property or district has mortgaged property")]
    DistrictHasMortgage,
    #[error("Maximum houses (hotel) already reached")]
    MaxHousesReached,
    #[error("Must build evenly across the district")]
    UnevenBuilding,
    #[error("No houses to sell on tile {0}")]
    NoHousesToSell(u8),
    #[error("Property {0} is already mortgaged")]
    PropertyAlreadyMortgaged(u8),
    #[error("Property {0} is not mortgaged")]
    PropertyNotMortgaged(u8),
    #[error("Cannot mortgage: property has buildings")]
    PropertyHasBuildings(u8),
    #[error("Not currently in auction")]
    NotInAuction,
    #[error("Bid {bid} must be greater than current bid {current}")]
    BidTooLow { bid: Money, current: Money },
    #[error("Invalid action: {0}")]
    InvalidAction(String),
}

pub fn apply_action(
    state: &GameState,
    player_id: PlayerId,
    action: &ClientMessage,
    rng: &mut impl Rng,
) -> Result<(GameState, Vec<ServerMessage>), GameRuleError> {
    if !state.is_game_started {
        return Err(GameRuleError::GameNotStarted);
    }
    if state.is_game_over {
        return Err(GameRuleError::GameOver);
    }

    let mut new_state = state.clone();
    let mut events = Vec::new();

    // Verify current player for turn-based actions
    match action {
        ClientMessage::JoinRoom { .. } | ClientMessage::StartGame | ClientMessage::AddBot => {
            return Err(GameRuleError::InvalidAction(
                "Game already in progress".to_string(),
            ));
        }
        ClientMessage::PlaceBid { .. } | ClientMessage::PassAuction => {
            if new_state.turn_phase != TurnPhase::Auction {
                return Err(GameRuleError::NotInAuction);
            }
            let in_auction = new_state
                .auction
                .as_ref()
                .map(|a| a.active_bidders.contains(&player_id))
                .unwrap_or(false);
            if !in_auction {
                return Err(GameRuleError::InvalidAction(
                    "Bạn không còn trong phiên đấu giá".to_string(),
                ));
            }
        }
        ClientMessage::RespondTrade { .. } => {
            if new_state.trade.as_ref().map(|t| t.to) != Some(player_id) {
                return Err(GameRuleError::InvalidAction(
                    "Không có đề nghị giao dịch nào gửi cho bạn".to_string(),
                ));
            }
        }
        _ => {
            let current = new_state
                .current_player()
                .ok_or(GameRuleError::GameNotStarted)?;
            if current.id != player_id {
                return Err(GameRuleError::NotCurrentPlayer(player_id));
            }
        }
    }

    if new_state.turn_phase == TurnPhase::Debt
        && !matches!(
            action,
            ClientMessage::SellHouse { .. }
                | ClientMessage::MortgageProperty { .. }
                | ClientMessage::PayDebt
                | ClientMessage::DeclareBankruptcy
        )
    {
        return Err(GameRuleError::InvalidAction(
            "Cần trả nợ hoặc tuyên bố phá sản trước".to_string(),
        ));
    }

    match action {
        ClientMessage::RollDice => {
            clear_trade(&mut new_state, &mut events);
            if new_state.turn_phase != TurnPhase::WaitingForRoll {
                return Err(GameRuleError::InvalidPhase {
                    expected: "WaitingForRoll",
                    actual: new_state.turn_phase,
                });
            }

            let die1 = rng.gen_range(1..=6);
            let die2 = rng.gen_range(1..=6);
            let dice_sum = die1 + die2;
            let is_doubles = die1 == die2;
            new_state.last_dice = (die1, die2);

            let in_jail = {
                let current_player = new_state
                    .current_player()
                    .ok_or(GameRuleError::GameNotStarted)?;
                current_player.in_jail
            };

            if in_jail {
                if is_doubles {
                    new_state.doubles_count = 0;
                    let (_old_pos, new_pos) = {
                        let current_player = new_state.current_player_mut().unwrap();
                        current_player.in_jail = false;
                        current_player.turns_in_jail = 0;
                        let old = current_player.position;
                        let new = (old + dice_sum) % 40;
                        current_player.position = new;
                        (old, new)
                    };

                    events.push(ServerMessage::DiceRolled {
                        player_id,
                        dice: (die1, die2),
                        new_tile_index: new_pos,
                        is_doubles: true,
                    });

                    resolve_landing(
                        &mut new_state,
                        player_id,
                        new_pos,
                        dice_sum,
                        &mut events,
                        rng,
                    );
                } else {
                    let (turns, cash, cur_pos) = {
                        let current_player = new_state.current_player_mut().unwrap();
                        current_player.turns_in_jail += 1;
                        (
                            current_player.turns_in_jail,
                            current_player.cash,
                            current_player.position,
                        )
                    };

                    if turns >= 3 {
                        let bail = Money::from_vnd(JAIL_BAIL_VND);
                        if cash >= bail {
                            let new_pos = {
                                let current_player = new_state.current_player_mut().unwrap();
                                current_player.cash -= bail;
                                current_player.in_jail = false;
                                current_player.turns_in_jail = 0;
                                let old = current_player.position;
                                let new = (old + dice_sum) % 40;
                                current_player.position = new;
                                new
                            };

                            events.push(ServerMessage::DiceRolled {
                                player_id,
                                dice: (die1, die2),
                                new_tile_index: new_pos,
                                is_doubles: false,
                            });

                            resolve_landing(
                                &mut new_state,
                                player_id,
                                new_pos,
                                dice_sum,
                                &mut events,
                                rng,
                            );
                        } else {
                            // Không đủ tiền bảo lãnh: ở lại, gom tiền hoặc phá sản
                            events.push(ServerMessage::DiceRolled {
                                player_id,
                                dice: (die1, die2),
                                new_tile_index: cur_pos,
                                is_doubles: false,
                            });
                            new_state.turn_phase = TurnPhase::TurnEnded;
                            charge(&mut new_state, player_id, vec![], bail, None, &mut events);
                        }
                    } else {
                        events.push(ServerMessage::DiceRolled {
                            player_id,
                            dice: (die1, die2),
                            new_tile_index: cur_pos,
                            is_doubles: false,
                        });
                        new_state.turn_phase = TurnPhase::TurnEnded;
                    }
                }
            } else {
                if is_doubles {
                    new_state.doubles_count += 1;
                    if new_state.doubles_count >= 3 {
                        let current_player = new_state.current_player_mut().unwrap();
                        current_player.position = 10;
                        current_player.in_jail = true;
                        current_player.turns_in_jail = 0;
                        new_state.doubles_count = 0;
                        new_state.turn_phase = TurnPhase::TurnEnded;

                        events.push(ServerMessage::DiceRolled {
                            player_id,
                            dice: (die1, die2),
                            new_tile_index: 10,
                            is_doubles: true,
                        });
                        return Ok((new_state, events));
                    }
                } else {
                    new_state.doubles_count = 0;
                }

                let new_pos = {
                    let current_player = new_state.current_player_mut().unwrap();
                    let old_pos = current_player.position;
                    let new_p = (old_pos + dice_sum) % 40;
                    current_player.position = new_p;

                    if new_p < old_pos {
                        let pass_go_reward = Money::from_vnd(PASS_GO_REWARD_VND);
                        current_player.cash += pass_go_reward;
                    }
                    new_p
                };

                events.push(ServerMessage::DiceRolled {
                    player_id,
                    dice: (die1, die2),
                    new_tile_index: new_pos,
                    is_doubles,
                });

                resolve_landing(
                    &mut new_state,
                    player_id,
                    new_pos,
                    dice_sum,
                    &mut events,
                    rng,
                );
            }
        }

        ClientMessage::BuyProperty => {
            if new_state.turn_phase != TurnPhase::TileActionPending {
                return Err(GameRuleError::InvalidPhase {
                    expected: "TileActionPending",
                    actual: new_state.turn_phase,
                });
            }

            let player = new_state
                .find_player(player_id)
                .ok_or(GameRuleError::NotCurrentPlayer(player_id))?;
            let pos = player.position;
            let board = get_board();
            let tile = &board[pos as usize];

            let cost = match &tile.kind {
                TileKind::Property(p) => p.cost,
                TileKind::Station(s) => s.cost,
                TileKind::Utility(u) => u.cost,
                _ => return Err(GameRuleError::TileNotPurchasable(pos)),
            };

            if new_state.properties[pos as usize].owner.is_some() {
                return Err(GameRuleError::TileAlreadyOwned(pos));
            }

            if player.cash < cost {
                return Err(GameRuleError::InsufficientFunds {
                    needed: cost,
                    available: player.cash,
                });
            }

            let player_mut = new_state.find_player_mut(player_id).unwrap();
            player_mut.cash -= cost;
            new_state.properties[pos as usize].owner = Some(player_id);

            events.push(ServerMessage::PropertyAcquired {
                player_id,
                tile_index: pos,
                cost,
            });

            new_state.turn_phase = TurnPhase::TurnEnded;
        }

        ClientMessage::DeclineProperty => {
            if new_state.turn_phase != TurnPhase::TileActionPending {
                return Err(GameRuleError::InvalidPhase {
                    expected: "TileActionPending",
                    actual: new_state.turn_phase,
                });
            }

            if new_state.config.enable_auction {
                let player = new_state.find_player(player_id).unwrap();
                let pos = player.position;

                let active_bidders: Vec<PlayerId> = new_state
                    .players
                    .iter()
                    .filter(|p| !p.is_bankrupt)
                    .map(|p| p.id)
                    .collect();

                new_state.turn_phase = TurnPhase::Auction;
                new_state.auction = Some(crate::state::AuctionSession {
                    tile_index: pos,
                    current_bid: Money::ZERO,
                    highest_bidder: None,
                    active_bidders,
                });

                events.push(ServerMessage::AuctionStarted {
                    tile_index: pos,
                    opening_bid: Money::ZERO,
                });
            } else {
                new_state.turn_phase = TurnPhase::TurnEnded;
            }
        }

        ClientMessage::PlaceBid { amount } => {
            let (current_bid, leader) = {
                let auction = new_state
                    .auction
                    .as_ref()
                    .ok_or(GameRuleError::NotInAuction)?;
                (auction.current_bid, auction.highest_bidder)
            };
            if leader == Some(player_id) {
                return Err(GameRuleError::InvalidAction(
                    "Bạn đang trả giá cao nhất".to_string(),
                ));
            }
            if *amount <= current_bid {
                return Err(GameRuleError::BidTooLow {
                    bid: *amount,
                    current: current_bid,
                });
            }
            let bidder = new_state
                .find_player(player_id)
                .ok_or(GameRuleError::NotCurrentPlayer(player_id))?;
            if bidder.cash < *amount {
                return Err(GameRuleError::InsufficientFunds {
                    needed: *amount,
                    available: bidder.cash,
                });
            }

            let auction = new_state.auction.as_mut().unwrap();
            auction.current_bid = *amount;
            auction.highest_bidder = Some(player_id);
            let only_one_left = auction.active_bidders.len() <= 1;
            events.push(ServerMessage::BidPlaced {
                bidder: player_id,
                amount: *amount,
            });
            if only_one_left {
                close_auction(&mut new_state, &mut events);
            }
        }

        ClientMessage::PassAuction => {
            let auction = new_state
                .auction
                .as_mut()
                .ok_or(GameRuleError::NotInAuction)?;
            if auction.highest_bidder == Some(player_id) {
                return Err(GameRuleError::InvalidAction(
                    "Bạn đang trả giá cao nhất".to_string(),
                ));
            }
            auction.active_bidders.retain(|&id| id != player_id);
            let remaining = auction.active_bidders.clone();
            let leader = auction.highest_bidder;
            // Kết thúc khi không còn ai ngoài người đang dẫn
            if remaining.is_empty() || (remaining.len() == 1 && leader == Some(remaining[0])) {
                close_auction(&mut new_state, &mut events);
            }
        }

        ClientMessage::BuildHouse { tile_index } => {
            let tile_idx = *tile_index as usize;
            let board = get_board();
            let tile = &board[tile_idx];

            let prop = match &tile.kind {
                TileKind::Property(p) => p,
                _ => return Err(GameRuleError::TileNotPurchasable(*tile_index)),
            };

            if new_state.properties[tile_idx].owner != Some(player_id) {
                return Err(GameRuleError::NotPropertyOwner(*tile_index));
            }

            if !new_state.owns_full_district(player_id, prop.district) {
                return Err(GameRuleError::MustOwnMonopolyToBuild);
            }

            let district_tiles = get_district_tiles(prop.district);
            if district_tiles
                .iter()
                .any(|&t| new_state.properties[t as usize].is_mortgaged)
            {
                return Err(GameRuleError::DistrictHasMortgage);
            }

            let current_houses = new_state.properties[tile_idx].house_count;
            if current_houses >= 5 {
                return Err(GameRuleError::MaxHousesReached);
            }

            let min_houses = district_tiles
                .iter()
                .map(|&t| new_state.properties[t as usize].house_count)
                .min()
                .unwrap_or(0);

            if current_houses > min_houses {
                return Err(GameRuleError::UnevenBuilding);
            }

            let house_cost = prop.house_cost;
            let player = new_state.find_player_mut(player_id).unwrap();
            if player.cash < house_cost {
                return Err(GameRuleError::InsufficientFunds {
                    needed: house_cost,
                    available: player.cash,
                });
            }

            player.cash -= house_cost;
            new_state.properties[tile_idx].house_count += 1;

            events.push(ServerMessage::HouseModified {
                tile_index: *tile_index,
                house_count: new_state.properties[tile_idx].house_count,
            });
        }

        ClientMessage::SellHouse { tile_index } => {
            let tile_idx = *tile_index as usize;
            let board = get_board();
            let tile = &board[tile_idx];

            let prop = match &tile.kind {
                TileKind::Property(p) => p,
                _ => return Err(GameRuleError::TileNotPurchasable(*tile_index)),
            };

            if new_state.properties[tile_idx].owner != Some(player_id) {
                return Err(GameRuleError::NotPropertyOwner(*tile_index));
            }

            let current_houses = new_state.properties[tile_idx].house_count;
            if current_houses == 0 {
                return Err(GameRuleError::NoHousesToSell(*tile_index));
            }

            let district_tiles = get_district_tiles(prop.district);
            let max_houses = district_tiles
                .iter()
                .map(|&t| new_state.properties[t as usize].house_count)
                .max()
                .unwrap_or(0);

            if current_houses < max_houses {
                return Err(GameRuleError::UnevenBuilding);
            }

            let refund = Money(prop.house_cost.0 / 2);
            let player = new_state.find_player_mut(player_id).unwrap();
            player.cash += refund;
            new_state.properties[tile_idx].house_count -= 1;

            events.push(ServerMessage::HouseModified {
                tile_index: *tile_index,
                house_count: new_state.properties[tile_idx].house_count,
            });
        }

        ClientMessage::MortgageProperty { tile_index } => {
            let tile_idx = *tile_index as usize;
            if new_state.properties[tile_idx].owner != Some(player_id) {
                return Err(GameRuleError::NotPropertyOwner(*tile_index));
            }
            if new_state.properties[tile_idx].is_mortgaged {
                return Err(GameRuleError::PropertyAlreadyMortgaged(*tile_index));
            }
            if new_state.properties[tile_idx].house_count > 0 {
                return Err(GameRuleError::PropertyHasBuildings(*tile_index));
            }

            let board = get_board();
            let mortgage_val = match &board[tile_idx].kind {
                TileKind::Property(p) => {
                    let d_tiles = get_district_tiles(p.district);
                    if d_tiles
                        .iter()
                        .any(|&t| new_state.properties[t as usize].house_count > 0)
                    {
                        return Err(GameRuleError::PropertyHasBuildings(*tile_index));
                    }
                    p.mortgage_value
                }
                TileKind::Station(s) => s.mortgage_value,
                TileKind::Utility(u) => u.mortgage_value,
                _ => return Err(GameRuleError::TileNotPurchasable(*tile_index)),
            };

            new_state.properties[tile_idx].is_mortgaged = true;
            let player = new_state.find_player_mut(player_id).unwrap();
            player.cash += mortgage_val;

            events.push(ServerMessage::PropertyMortgageChanged {
                tile_index: *tile_index,
                is_mortgaged: true,
            });
        }

        ClientMessage::UnmortgageProperty { tile_index } => {
            let tile_idx = *tile_index as usize;
            if new_state.properties[tile_idx].owner != Some(player_id) {
                return Err(GameRuleError::NotPropertyOwner(*tile_index));
            }
            if !new_state.properties[tile_idx].is_mortgaged {
                return Err(GameRuleError::PropertyNotMortgaged(*tile_index));
            }

            let board = get_board();
            let mortgage_val = match &board[tile_idx].kind {
                TileKind::Property(p) => p.mortgage_value,
                TileKind::Station(s) => s.mortgage_value,
                TileKind::Utility(u) => u.mortgage_value,
                _ => return Err(GameRuleError::TileNotPurchasable(*tile_index)),
            };

            let unmortgage_cost = Money((mortgage_val.0 * 11) / 10);
            let player = new_state.find_player_mut(player_id).unwrap();
            if player.cash < unmortgage_cost {
                return Err(GameRuleError::InsufficientFunds {
                    needed: unmortgage_cost,
                    available: player.cash,
                });
            }

            player.cash -= unmortgage_cost;
            new_state.properties[tile_idx].is_mortgaged = false;

            events.push(ServerMessage::PropertyMortgageChanged {
                tile_index: *tile_index,
                is_mortgaged: false,
            });
        }

        ClientMessage::PayJailBail => {
            let player = new_state
                .find_player_mut(player_id)
                .ok_or(GameRuleError::NotCurrentPlayer(player_id))?;
            if !player.in_jail {
                return Err(GameRuleError::PlayerNotInJail(player_id));
            }

            let bail = Money::from_vnd(JAIL_BAIL_VND);
            if player.cash < bail {
                return Err(GameRuleError::InsufficientFunds {
                    needed: bail,
                    available: player.cash,
                });
            }

            player.cash -= bail;
            player.in_jail = false;
            player.turns_in_jail = 0;
        }

        ClientMessage::UseJailCard => {
            let player = new_state
                .find_player_mut(player_id)
                .ok_or(GameRuleError::NotCurrentPlayer(player_id))?;
            if !player.in_jail {
                return Err(GameRuleError::PlayerNotInJail(player_id));
            }
            if player.get_out_of_jail_cards == 0 {
                return Err(GameRuleError::NoJailCard);
            }

            player.get_out_of_jail_cards -= 1;
            player.in_jail = false;
            player.turns_in_jail = 0;
        }

        ClientMessage::DeclareBankruptcy => {
            let creditors = new_state
                .debt
                .take()
                .filter(|d| d.debtor == player_id)
                .map(|d| d.creditors)
                .unwrap_or_default();
            let creditor = if creditors.len() == 1 { Some(creditors[0]) } else { None };
            // Tiền mặt còn lại chuyển cho chủ nợ
            let cash = new_state.find_player(player_id).map(|p| p.cash).unwrap_or(Money::ZERO);
            if let Some(c) = creditor {
                if let Some(cp) = new_state.find_player_mut(c) {
                    cp.cash += cash;
                }
            }
            handle_bankruptcy(&mut new_state, player_id, creditor, &mut events);
        }

        ClientMessage::PayDebt => {
            let debt = match new_state.debt.clone() {
                Some(d) if d.debtor == player_id && new_state.turn_phase == TurnPhase::Debt => d,
                _ => return Err(GameRuleError::InvalidAction("Bạn không có khoản nợ nào".to_string())),
            };
            let cash = new_state.find_player(player_id).map(|p| p.cash).unwrap_or(Money::ZERO);
            if cash < debt.amount {
                return Err(GameRuleError::InsufficientFunds {
                    needed: debt.amount,
                    available: cash,
                });
            }
            settle(&mut new_state, player_id, &debt.creditors, debt.amount, debt.tile_index, &mut events);
            new_state.debt = None;
            new_state.turn_phase = TurnPhase::TurnEnded;
        }

        ClientMessage::ProposeTrade {
            to,
            give_tiles,
            give_cash,
            take_tiles,
            take_cash,
        } => {
            if !matches!(new_state.turn_phase, TurnPhase::WaitingForRoll | TurnPhase::TurnEnded) {
                return Err(GameRuleError::InvalidAction(
                    "Chỉ giao dịch được trước khi lắc hoặc sau khi đi xong".to_string(),
                ));
            }
            if new_state.trade.is_some() {
                return Err(GameRuleError::InvalidAction("Đang có một đề nghị chờ trả lời".to_string()));
            }
            let offer = TradeOffer {
                from: player_id,
                to: *to,
                give_tiles: give_tiles.clone(),
                give_cash: *give_cash,
                take_tiles: take_tiles.clone(),
                take_cash: *take_cash,
            };
            validate_trade(&new_state, &offer)?;
            new_state.trade = Some(offer);
            events.push(ServerMessage::TradeProposed {
                from: player_id,
                to: *to,
            });
        }

        ClientMessage::RespondTrade { accept } => {
            let offer = new_state.trade.take().unwrap();
            if *accept {
                validate_trade(&new_state, &offer)?;
                for &t in &offer.give_tiles {
                    new_state.properties[t as usize].owner = Some(offer.to);
                }
                for &t in &offer.take_tiles {
                    new_state.properties[t as usize].owner = Some(offer.from);
                }
                let net = offer.give_cash.0 - offer.take_cash.0;
                new_state.find_player_mut(offer.from).unwrap().cash -= Money(net);
                new_state.find_player_mut(offer.to).unwrap().cash += Money(net);
            }
            events.push(ServerMessage::TradeResolved {
                from: offer.from,
                to: offer.to,
                accepted: *accept,
            });
        }

        ClientMessage::CancelTrade => {
            if new_state.trade.as_ref().map(|t| t.from) != Some(player_id) {
                return Err(GameRuleError::InvalidAction("Không có đề nghị nào để rút".to_string()));
            }
            clear_trade(&mut new_state, &mut events);
        }

        ClientMessage::EndTurn => {
            clear_trade(&mut new_state, &mut events);
            if new_state.turn_phase != TurnPhase::TurnEnded {
                return Err(GameRuleError::InvalidPhase {
                    expected: "TurnEnded",
                    actual: new_state.turn_phase,
                });
            }

            let current = new_state.find_player(player_id).unwrap();
            if new_state.doubles_count > 0 && !current.in_jail && new_state.doubles_count < 3 {
                new_state.turn_phase = TurnPhase::WaitingForRoll;
                events.push(ServerMessage::TurnStarted {
                    active_player: player_id,
                    time_limit_seconds: 120,
                });
            } else {
                new_state.doubles_count = 0;
                advance_turn(&mut new_state, &mut events);
            }
        }
        ClientMessage::ConfigureGame {
            enable_auction,
            max_players,
        } => {
            new_state.config.enable_auction = *enable_auction;
            if let Some(mp) = max_players {
                let min_allowed = new_state.players.len().max(2) as u8;
                new_state.config.max_players = (*mp).clamp(2, 4).max(min_allowed);
            }
            if !*enable_auction && new_state.turn_phase == TurnPhase::Auction {
                new_state.auction = None;
                new_state.turn_phase = TurnPhase::TurnEnded;
            }
        }
        ClientMessage::JoinRoom { .. } | ClientMessage::StartGame | ClientMessage::AddBot => {
            unreachable!()
        }
    }

    Ok((new_state, events))
}

fn resolve_landing(
    state: &mut GameState,
    player_id: PlayerId,
    tile_index: u8,
    dice_sum: u8,
    events: &mut Vec<ServerMessage>,
    rng: &mut impl Rng,
) {
    let board = get_board();
    let tile = &board[tile_index as usize];

    match &tile.kind {
        TileKind::Go | TileKind::FreeParking | TileKind::Jail => {
            state.turn_phase = TurnPhase::TurnEnded;
        }
        TileKind::GoToJail => {
            let player = state.find_player_mut(player_id).unwrap();
            player.position = 10;
            player.in_jail = true;
            player.turns_in_jail = 0;
            state.doubles_count = 0;
            state.turn_phase = TurnPhase::TurnEnded;
        }
        TileKind::Tax(t) => {
            state.turn_phase = TurnPhase::TurnEnded;
            charge(state, player_id, vec![], t.amount, Some(tile_index), events);
        }
        TileKind::Property(p) => {
            let prop_state = state.properties[tile_index as usize];
            if let Some(owner_id) = prop_state.owner {
                if owner_id == player_id || prop_state.is_mortgaged {
                    state.turn_phase = TurnPhase::TurnEnded;
                } else {
                    let rent = if prop_state.house_count > 0 {
                        p.rent_table[prop_state.house_count as usize]
                    } else if state.owns_full_district(owner_id, p.district) {
                        Money(p.rent_table[0].0 * 2)
                    } else {
                        p.rent_table[0]
                    };

                    state.turn_phase = TurnPhase::TurnEnded;
                    pay_rent(state, player_id, owner_id, tile_index, rent, events);
                }
            } else {
                state.turn_phase = TurnPhase::TileActionPending;
            }
        }
        TileKind::Station(_) => {
            let prop_state = state.properties[tile_index as usize];
            if let Some(owner_id) = prop_state.owner {
                if owner_id == player_id || prop_state.is_mortgaged {
                    state.turn_phase = TurnPhase::TurnEnded;
                } else {
                    let stations = state.stations_owned_by(owner_id);
                    let multiplier = 1i64 << (stations.saturating_sub(1));
                    let rent = Money::from_vnd(250_000 * multiplier);

                    state.turn_phase = TurnPhase::TurnEnded;
                    pay_rent(state, player_id, owner_id, tile_index, rent, events);
                }
            } else {
                state.turn_phase = TurnPhase::TileActionPending;
            }
        }
        TileKind::Utility(_) => {
            let prop_state = state.properties[tile_index as usize];
            if let Some(owner_id) = prop_state.owner {
                if owner_id == player_id || prop_state.is_mortgaged {
                    state.turn_phase = TurnPhase::TurnEnded;
                } else {
                    let utils = state.utilities_owned_by(owner_id);
                    let mult = if utils >= 2 { 10i64 } else { 4i64 };
                    let rent = Money::from_vnd(mult * (dice_sum as i64) * 1_000);

                    state.turn_phase = TurnPhase::TurnEnded;
                    pay_rent(state, player_id, owner_id, tile_index, rent, events);
                }
            } else {
                state.turn_phase = TurnPhase::TileActionPending;
            }
        }
        TileKind::Chance => {
            if state.chance_deck.is_empty() {
                state.chance_deck = crate::cards::get_chance_deck();
            }
            let card_idx = rng.gen_range(0..state.chance_deck.len());
            let card = state.chance_deck.remove(card_idx);
            apply_card(state, player_id, &card, dice_sum, events, rng);
        }
        TileKind::CommunityChest => {
            if state.community_chest_deck.is_empty() {
                state.community_chest_deck = crate::cards::get_community_chest_deck();
            }
            let card_idx = rng.gen_range(0..state.community_chest_deck.len());
            let card = state.community_chest_deck.remove(card_idx);
            apply_card(state, player_id, &card, dice_sum, events, rng);
        }
    }
}

fn apply_card(
    state: &mut GameState,
    player_id: PlayerId,
    card: &crate::cards::Card,
    dice_sum: u8,
    events: &mut Vec<ServerMessage>,
    rng: &mut impl Rng,
) {
    events.push(ServerMessage::CardDrawn {
        player_id,
        deck: card.deck.to_string(),
        description: card.title.clone(),
        effect_summary: card.description.clone(),
    });

    match &card.effect {
        CardEffect::CashDelta(amount) => {
            state.turn_phase = TurnPhase::TurnEnded;
            if *amount >= Money::ZERO {
                state.find_player_mut(player_id).unwrap().cash += *amount;
            } else {
                charge(state, player_id, vec![], Money(-amount.0), None, events);
            }
        }
        CardEffect::MoveTo(dest) => {
            let target_pos = *dest;
            let player = state.find_player_mut(player_id).unwrap();
            let old_pos = player.position;
            player.position = target_pos;
            if target_pos < old_pos && target_pos != 10 {
                player.cash += Money::from_vnd(PASS_GO_REWARD_VND);
            }
            resolve_landing(state, player_id, target_pos, dice_sum, events, rng);
        }
        CardEffect::GoToJail => {
            let player = state.find_player_mut(player_id).unwrap();
            player.position = 10;
            player.in_jail = true;
            player.turns_in_jail = 0;
            state.doubles_count = 0;
            state.turn_phase = TurnPhase::TurnEnded;
        }
        CardEffect::GetOutOfJailFree => {
            let player = state.find_player_mut(player_id).unwrap();
            player.get_out_of_jail_cards += 1;
            state.turn_phase = TurnPhase::TurnEnded;
        }
        CardEffect::BuildingRepairs { house, hotel } => {
            let mut total = Money::ZERO;
            for prop in &state.properties {
                if prop.owner == Some(player_id) {
                    if prop.house_count == 5 {
                        total += *hotel;
                    } else if prop.house_count > 0 {
                        total += Money(house.0 * prop.house_count as i64);
                    }
                }
            }
            state.turn_phase = TurnPhase::TurnEnded;
            charge(state, player_id, vec![], total, None, events);
        }
        CardEffect::CollectFromAll(amount) => {
            let active_others: Vec<PlayerId> = state
                .players
                .iter()
                .filter(|p| !p.is_bankrupt && p.id != player_id)
                .map(|p| p.id)
                .collect();

            let mut collected = Money::ZERO;
            for other_id in active_others {
                let other = state.find_player_mut(other_id).unwrap();
                let can_pay = if other.cash >= *amount {
                    other.cash -= *amount;
                    *amount
                } else {
                    let avail = other.cash;
                    other.cash = Money::ZERO;
                    avail
                };
                collected += can_pay;
            }

            let player = state.find_player_mut(player_id).unwrap();
            player.cash += collected;
            state.turn_phase = TurnPhase::TurnEnded;
        }
        CardEffect::PayToAll(amount) => {
            let active_others: Vec<PlayerId> = state
                .players
                .iter()
                .filter(|p| !p.is_bankrupt && p.id != player_id)
                .map(|p| p.id)
                .collect();

            let total_due = Money(amount.0 * active_others.len() as i64);
            state.turn_phase = TurnPhase::TurnEnded;
            charge(state, player_id, active_others, total_due, None, events);
        }
    }
}

fn pay_rent(
    state: &mut GameState,
    payer_id: PlayerId,
    recipient_id: PlayerId,
    tile_index: u8,
    amount: Money,
    events: &mut Vec<ServerMessage>,
) {
    charge(state, payer_id, vec![recipient_id], amount, Some(tile_index), events);
}

/// Chuyển `amount` từ `payer` cho các chủ nợ (chia đều; rỗng = ngân hàng).
fn settle(
    state: &mut GameState,
    payer_id: PlayerId,
    creditors: &[PlayerId],
    amount: Money,
    tile_index: Option<u8>,
    events: &mut Vec<ServerMessage>,
) {
    state.find_player_mut(payer_id).unwrap().cash -= amount;
    if creditors.is_empty() {
        return;
    }
    let share = Money(amount.0 / creditors.len() as i64);
    for &c in creditors {
        if let Some(cp) = state.find_player_mut(c) {
            cp.cash += share;
        }
    }
    if let (Some(t), [single]) = (tile_index, creditors) {
        if matches!(get_board()[t as usize].kind, TileKind::Property(_) | TileKind::Station(_) | TileKind::Utility(_)) {
            events.push(ServerMessage::RentPaid {
                payer: payer_id,
                recipient: *single,
                tile_index: t,
                amount,
            });
        }
    }
}

/// Thu tiền của người chơi. Đủ tiền mặt thì trả ngay; thiếu nhưng còn tài sản thì
/// chuyển sang pha `Debt` để gom tiền; không thể gom đủ thì phá sản.
/// Người gọi phải đặt `turn_phase` mong muốn (thường là `TurnEnded`) trước khi gọi.
pub fn charge(
    state: &mut GameState,
    payer_id: PlayerId,
    creditors: Vec<PlayerId>,
    amount: Money,
    tile_index: Option<u8>,
    events: &mut Vec<ServerMessage>,
) {
    if amount <= Money::ZERO {
        return;
    }
    let cash = state.find_player(payer_id).map(|p| p.cash).unwrap_or(Money::ZERO);
    if cash >= amount {
        settle(state, payer_id, &creditors, amount, tile_index, events);
        return;
    }
    if cash + state.liquidation_value(payer_id) >= amount {
        events.push(ServerMessage::DebtIncurred {
            debtor: payer_id,
            creditor: if creditors.len() == 1 { Some(creditors[0]) } else { None },
            amount,
        });
        state.debt = Some(DebtSession {
            debtor: payer_id,
            creditors,
            amount,
            tile_index,
        });
        state.turn_phase = TurnPhase::Debt;
        return;
    }
    // Không thể gom đủ: trả hết tiền mặt rồi phá sản
    let creditor = if creditors.len() == 1 { Some(creditors[0]) } else { None };
    if cash > Money::ZERO {
        settle(state, payer_id, &creditors, cash, tile_index, events);
    }
    handle_bankruptcy(state, payer_id, creditor, events);
}

/// Đóng phiên đấu giá: người trả giá cao nhất mua đất; không ai trả thì đất vẫn của ngân hàng.
pub fn close_auction(state: &mut GameState, events: &mut Vec<ServerMessage>) {
    let Some(auction) = state.auction.take() else {
        return;
    };
    match auction.highest_bidder {
        Some(winner) if auction.current_bid > Money::ZERO => {
            if let Some(p) = state.find_player_mut(winner) {
                p.cash -= auction.current_bid;
            }
            state.properties[auction.tile_index as usize].owner = Some(winner);
            events.push(ServerMessage::AuctionConcluded {
                winner: Some(winner),
                final_price: auction.current_bid,
            });
            events.push(ServerMessage::PropertyAcquired {
                player_id: winner,
                tile_index: auction.tile_index,
                cost: auction.current_bid,
            });
        }
        _ => events.push(ServerMessage::AuctionConcluded {
            winner: None,
            final_price: Money::ZERO,
        }),
    }
    state.turn_phase = TurnPhase::TurnEnded;
}

fn clear_trade(state: &mut GameState, events: &mut Vec<ServerMessage>) {
    if let Some(t) = state.trade.take() {
        events.push(ServerMessage::TradeResolved {
            from: t.from,
            to: t.to,
            accepted: false,
        });
    }
}

fn validate_trade(state: &GameState, t: &TradeOffer) -> Result<(), GameRuleError> {
    let invalid = |m: &str| Err(GameRuleError::InvalidAction(m.to_string()));
    if t.from == t.to {
        return invalid("Không thể giao dịch với chính mình");
    }
    let Some(to) = state.find_player(t.to) else {
        return invalid("Người nhận không tồn tại");
    };
    if to.is_bankrupt {
        return invalid("Người nhận đã phá sản");
    }
    if t.give_cash < Money::ZERO || t.take_cash < Money::ZERO {
        return invalid("Số tiền không hợp lệ");
    }
    if t.give_tiles.is_empty() && t.take_tiles.is_empty() && t.give_cash == Money::ZERO && t.take_cash == Money::ZERO {
        return invalid("Đề nghị đang trống");
    }
    let board = get_board();
    let check_tiles = |tiles: &[u8], owner: PlayerId| -> Result<(), GameRuleError> {
        for &tile in tiles {
            let idx = tile as usize;
            if idx >= 40 || state.properties[idx].owner != Some(owner) {
                return Err(GameRuleError::NotPropertyOwner(tile));
            }
            if let TileKind::Property(p) = &board[idx].kind {
                if get_district_tiles(p.district)
                    .iter()
                    .any(|&d| state.properties[d as usize].house_count > 0)
                {
                    return Err(GameRuleError::PropertyHasBuildings(tile));
                }
            }
        }
        Ok(())
    };
    check_tiles(&t.give_tiles, t.from)?;
    check_tiles(&t.take_tiles, t.to)?;
    let from_cash = state.find_player(t.from).map(|p| p.cash).unwrap_or(Money::ZERO);
    if from_cash < t.give_cash {
        return Err(GameRuleError::InsufficientFunds { needed: t.give_cash, available: from_cash });
    }
    if to.cash < t.take_cash {
        return Err(GameRuleError::InsufficientFunds { needed: t.take_cash, available: to.cash });
    }
    Ok(())
}

pub fn handle_bankruptcy(
    state: &mut GameState,
    bankrupt_id: PlayerId,
    creditor_id: Option<PlayerId>,
    events: &mut Vec<ServerMessage>,
) {
    let bankrupt_player = state.find_player_mut(bankrupt_id).unwrap();
    bankrupt_player.is_bankrupt = true;
    bankrupt_player.cash = Money::ZERO;
    state.debt = None;
    state.doubles_count = 0;
    if state
        .trade
        .as_ref()
        .map(|t| t.from == bankrupt_id || t.to == bankrupt_id)
        .unwrap_or(false)
    {
        state.trade = None;
    }

    events.push(ServerMessage::PlayerBankrupt {
        player_id: bankrupt_id,
    });

    for prop in state.properties.iter_mut() {
        if prop.owner == Some(bankrupt_id) {
            if let Some(creditor) = creditor_id {
                prop.owner = Some(creditor);
            } else {
                prop.owner = None;
                prop.house_count = 0;
                prop.is_mortgaged = false;
            }
        }
    }

    let remaining: Vec<PlayerId> = state
        .players
        .iter()
        .filter(|p| !p.is_bankrupt)
        .map(|p| p.id)
        .collect();

    if remaining.is_empty() || (state.players.len() > 1 && remaining.len() <= 1) {
        state.is_game_over = true;
        state.winner = remaining.first().copied();
    } else {
        advance_turn(state, events);
    }
}

pub fn advance_turn(state: &mut GameState, events: &mut Vec<ServerMessage>) {
    let active_count = state.players.iter().filter(|p| !p.is_bankrupt).count();
    if active_count == 0 || (state.players.len() > 1 && active_count <= 1) {
        state.is_game_over = true;
        state.winner = state.players.iter().find(|p| !p.is_bankrupt).map(|p| p.id);
        return;
    }

    loop {
        state.current_player_index = (state.current_player_index + 1) % state.turn_order.len();
        let next_id = state.turn_order[state.current_player_index];
        let next_player = state.find_player(next_id).unwrap();
        if !next_player.is_bankrupt {
            state.turn_phase = TurnPhase::WaitingForRoll;
            events.push(ServerMessage::TurnStarted {
                active_player: next_id,
                time_limit_seconds: 120,
            });
            break;
        }
    }
}
