use crate::board::{get_board, get_district_tiles, TileKind};
use crate::cards::CardEffect;
use crate::money::Money;
use crate::state::{GameState, TurnPhase, JAIL_BAIL_VND, PASS_GO_REWARD_VND};
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
        ClientMessage::JoinRoom { .. } => {
            return Err(GameRuleError::InvalidAction(
                "Game already in progress".to_string(),
            ));
        }
        ClientMessage::PlaceBid { .. } | ClientMessage::PassAuction => {
            if new_state.turn_phase != TurnPhase::Auction {
                return Err(GameRuleError::NotInAuction);
            }
        }
        ClientMessage::ConfigureGame { .. } => {}
        _ => {
            let current = new_state
                .current_player()
                .ok_or(GameRuleError::GameNotStarted)?;
            if current.id != player_id {
                return Err(GameRuleError::NotCurrentPlayer(player_id));
            }
        }
    }

    match action {
        ClientMessage::RollDice => {
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
                            handle_bankruptcy(&mut new_state, player_id, None, &mut events);
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
            let current_bid = {
                let auction = new_state
                    .auction
                    .as_ref()
                    .ok_or(GameRuleError::NotInAuction)?;
                auction.current_bid
            };

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

            let auction = new_state
                .auction
                .as_mut()
                .ok_or(GameRuleError::NotInAuction)?;
            auction.current_bid = *amount;
            auction.highest_bidder = Some(player_id);

            events.push(ServerMessage::BidPlaced {
                bidder: player_id,
                amount: *amount,
            });

            if auction.active_bidders.len() <= 1 {
                let final_price = auction.current_bid;
                let tile_idx = auction.tile_index;
                if let Some(winner_player) = new_state.find_player_mut(player_id) {
                    winner_player.cash -= final_price;
                    new_state.properties[tile_idx as usize].owner = Some(player_id);
                }
                events.push(ServerMessage::AuctionConcluded {
                    winner: Some(player_id),
                    final_price,
                });
                events.push(ServerMessage::PropertyAcquired {
                    player_id,
                    tile_index: tile_idx,
                    cost: final_price,
                });
                new_state.auction = None;
                new_state.turn_phase = TurnPhase::TurnEnded;
            }
        }

        ClientMessage::PassAuction => {
            let auction = new_state
                .auction
                .as_mut()
                .ok_or(GameRuleError::NotInAuction)?;

            auction.active_bidders.retain(|&id| id != player_id);

            if auction.active_bidders.len() <= 1 {
                let winner = auction.highest_bidder;
                let final_price = auction.current_bid;
                let tile_idx = auction.tile_index;

                if let Some(winner_id) = winner {
                    if let Some(winner_player) = new_state.find_player_mut(winner_id) {
                        winner_player.cash -= final_price;
                        new_state.properties[tile_idx as usize].owner = Some(winner_id);
                    }
                    events.push(ServerMessage::AuctionConcluded {
                        winner: Some(winner_id),
                        final_price,
                    });
                    events.push(ServerMessage::PropertyAcquired {
                        player_id: winner_id,
                        tile_index: tile_idx,
                        cost: final_price,
                    });
                } else {
                    events.push(ServerMessage::AuctionConcluded {
                        winner: None,
                        final_price: Money::ZERO,
                    });
                }

                new_state.auction = None;
                new_state.turn_phase = TurnPhase::TurnEnded;
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
            handle_bankruptcy(&mut new_state, player_id, None, &mut events);
        }

        ClientMessage::EndTurn => {
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
                    time_limit_seconds: 30,
                });
            } else {
                new_state.doubles_count = 0;
                advance_turn(&mut new_state, &mut events);
            }
        }
        ClientMessage::ConfigureGame { enable_auction } => {
            new_state.config.enable_auction = *enable_auction;
            if !*enable_auction && new_state.turn_phase == TurnPhase::Auction {
                new_state.auction = None;
                new_state.turn_phase = TurnPhase::TurnEnded;
            }
        }
        ClientMessage::JoinRoom { .. } => unreachable!(),
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
            let tax_amount = t.amount;
            let player = state.find_player_mut(player_id).unwrap();
            if player.cash >= tax_amount {
                player.cash -= tax_amount;
                state.turn_phase = TurnPhase::TurnEnded;
            } else {
                handle_bankruptcy(state, player_id, None, events);
            }
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

                    pay_rent(state, player_id, owner_id, tile_index, rent, events);
                    state.turn_phase = TurnPhase::TurnEnded;
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

                    pay_rent(state, player_id, owner_id, tile_index, rent, events);
                    state.turn_phase = TurnPhase::TurnEnded;
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

                    pay_rent(state, player_id, owner_id, tile_index, rent, events);
                    state.turn_phase = TurnPhase::TurnEnded;
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
            let player = state.find_player_mut(player_id).unwrap();
            player.cash += *amount;
            if player.cash < Money::ZERO {
                handle_bankruptcy(state, player_id, None, events);
            } else {
                state.turn_phase = TurnPhase::TurnEnded;
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
            let player = state.find_player_mut(player_id).unwrap();
            if player.cash >= total {
                player.cash -= total;
                state.turn_phase = TurnPhase::TurnEnded;
            } else {
                handle_bankruptcy(state, player_id, None, events);
            }
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
            let player = state.find_player_mut(player_id).unwrap();

            if player.cash >= total_due {
                player.cash -= total_due;
                for other_id in active_others {
                    let other = state.find_player_mut(other_id).unwrap();
                    other.cash += *amount;
                }
                state.turn_phase = TurnPhase::TurnEnded;
            } else {
                handle_bankruptcy(state, player_id, None, events);
            }
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
    let payer = state.find_player_mut(payer_id).unwrap();
    if payer.cash >= amount {
        payer.cash -= amount;
        let recipient = state.find_player_mut(recipient_id).unwrap();
        recipient.cash += amount;

        events.push(ServerMessage::RentPaid {
            payer: payer_id,
            recipient: recipient_id,
            tile_index,
            amount,
        });
    } else {
        let partial = payer.cash;
        payer.cash = Money::ZERO;
        let recipient = state.find_player_mut(recipient_id).unwrap();
        recipient.cash += partial;

        events.push(ServerMessage::RentPaid {
            payer: payer_id,
            recipient: recipient_id,
            tile_index,
            amount: partial,
        });

        handle_bankruptcy(state, payer_id, Some(recipient_id), events);
    }
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
                time_limit_seconds: 30,
            });
            break;
        }
    }
}
