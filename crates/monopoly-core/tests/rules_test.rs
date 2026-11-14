mod helpers;

use helpers::{create_seeded_rng, create_test_game};
use monopoly_core::board::District;
use monopoly_core::rules::{apply_action, handle_bankruptcy};
use monopoly_core::state::{TurnPhase, PASS_GO_REWARD_VND};
use monopoly_proto::{ClientMessage, Money, ServerMessage};
use rand::Rng;

#[test]
fn test_money_math_overflow_and_formatting() {
    let m1 = Money::from_vnd(2_000_000);
    let m2 = Money::from_vnd(-500_000);
    assert_eq!(m1.to_string(), "2.000.000 đ");
    assert_eq!(m2.to_string(), "-500.000 đ");
}

#[test]
fn test_board_indexing_and_pass_go() {
    let (mut state, player_ids) = create_test_game();
    let p1_id = player_ids[0];

    state.find_player_mut(p1_id).unwrap().position = 38;
    let initial_cash = state.find_player(p1_id).unwrap().cash;

    let mut rng = create_seeded_rng(42);

    let old_pos = state.find_player(p1_id).unwrap().position;
    let (next_state, events) =
        apply_action(&state, p1_id, &ClientMessage::RollDice, &mut rng).unwrap();

    let new_pos = next_state.find_player(p1_id).unwrap().position;
    let new_cash = next_state.find_player(p1_id).unwrap().cash;

    if new_pos < old_pos {
        assert_eq!(new_cash, initial_cash + Money::from_vnd(PASS_GO_REWARD_VND));
    }
    assert!(events
        .iter()
        .any(|e| matches!(e, ServerMessage::DiceRolled { .. })));
}

#[test]
fn test_property_acquisition_and_ledger() {
    let (mut state, player_ids) = create_test_game();
    let p1_id = player_ids[0];

    state.find_player_mut(p1_id).unwrap().position = 39;
    state.find_player_mut(p1_id).unwrap().cash = Money::from_vnd(5_000_000);
    state.turn_phase = TurnPhase::TileActionPending;

    let mut rng = create_seeded_rng(1);
    let (next_state, events) =
        apply_action(&state, p1_id, &ClientMessage::BuyProperty, &mut rng).unwrap();

    assert_eq!(
        next_state.find_player(p1_id).unwrap().cash,
        Money::from_vnd(1_000_000)
    );
    assert_eq!(next_state.properties[39].owner, Some(p1_id));
    assert_eq!(next_state.turn_phase, TurnPhase::TurnEnded);

    assert!(events.iter().any(|e| matches!(
        e,
        ServerMessage::PropertyAcquired {
            tile_index: 39,
            cost,
            ..
        } if *cost == Money::from_vnd(4_000_000)
    )));
}

#[test]
fn test_monopoly_base_rent_doubling() {
    let (mut state, player_ids) = create_test_game();
    let landlord_id = player_ids[0];
    let renter_id = player_ids[1];

    state.properties[26].owner = Some(landlord_id);
    state.properties[27].owner = Some(landlord_id);
    state.properties[29].owner = Some(landlord_id);

    assert!(state.owns_full_district(landlord_id, District::District3));

    let renter_initial_cash = state.find_player(renter_id).unwrap().cash;
    let landlord_initial_cash = state.find_player(landlord_id).unwrap().cash;

    state.current_player_index = 1;
    state.find_player_mut(renter_id).unwrap().position = 20;
    state.turn_phase = TurnPhase::WaitingForRoll;

    let mut rng = create_seeded_rng(9);
    for s in 0..100 {
        let mut test_rng = create_seeded_rng(s);
        let d1: u8 = test_rng.gen_range(1..=6);
        let d2: u8 = test_rng.gen_range(1..=6);
        if d1 + d2 == 6 {
            println!("Found seed {}: d1={}, d2={}", s, d1, d2);
            rng = create_seeded_rng(s);
            break;
        }
    }

    let (next_state, events) =
        apply_action(&state, renter_id, &ClientMessage::RollDice, &mut rng).unwrap();
    println!(
        "Rolled in apply_action: last_dice={:?}",
        next_state.last_dice
    );

    assert_eq!(next_state.find_player(renter_id).unwrap().position, 26);
    let expected_rent = Money::from_vnd(440_000);
    assert_eq!(
        next_state.find_player(renter_id).unwrap().cash,
        renter_initial_cash - expected_rent
    );
    assert_eq!(
        next_state.find_player(landlord_id).unwrap().cash,
        landlord_initial_cash + expected_rent
    );

    assert!(events.iter().any(|e| matches!(
        e,
        ServerMessage::RentPaid {
            payer,
            recipient,
            tile_index: 26,
            amount,
        } if *payer == renter_id && *recipient == landlord_id && *amount == expected_rent
    )));
}

#[test]
fn test_station_escalating_rent() {
    let (mut state, player_ids) = create_test_game();
    let landlord_id = player_ids[0];

    state.properties[5].owner = Some(landlord_id);
    assert_eq!(state.stations_owned_by(landlord_id), 1);

    state.properties[15].owner = Some(landlord_id);
    assert_eq!(state.stations_owned_by(landlord_id), 2);

    state.properties[25].owner = Some(landlord_id);
    assert_eq!(state.stations_owned_by(landlord_id), 3);

    state.properties[35].owner = Some(landlord_id);
    assert_eq!(state.stations_owned_by(landlord_id), 4);
}

#[test]
fn test_utility_dice_multiplier_rent() {
    let (state, player_ids) = create_test_game();
    let landlord_id = player_ids[0];

    let mut state1 = state.clone();
    state1.properties[12].owner = Some(landlord_id);
    assert_eq!(state1.utilities_owned_by(landlord_id), 1);

    let mut state2 = state.clone();
    state2.properties[12].owner = Some(landlord_id);
    state2.properties[28].owner = Some(landlord_id);
    assert_eq!(state2.utilities_owned_by(landlord_id), 2);

    let dice_sum = 7i64;
    let rent1 = Money::from_vnd(4 * dice_sum * 1_000);
    let rent2 = Money::from_vnd(10 * dice_sum * 1_000);
    assert_eq!(rent1, Money::from_vnd(28_000));
    assert_eq!(rent2, Money::from_vnd(70_000));
}

#[test]
fn test_doubles_and_consecutive_jail() {
    let (state, player_ids) = create_test_game();
    let p1_id = player_ids[0];

    let mut doubles_seeds = Vec::new();
    for s in 0..500 {
        let mut test_rng = create_seeded_rng(s);
        let d1: u8 = test_rng.gen_range(1..=6);
        let d2: u8 = test_rng.gen_range(1..=6);
        if d1 == d2 {
            doubles_seeds.push((s, d1, d2));
            if doubles_seeds.len() >= 3 {
                break;
            }
        }
    }

    println!("Doubles seeds: {:?}", doubles_seeds);

    let mut rng1 = create_seeded_rng(doubles_seeds[0].0);
    let (state_after_roll1, events1) =
        apply_action(&state, p1_id, &ClientMessage::RollDice, &mut rng1).unwrap();
    println!(
        "Roll 1: doubles_count={}, last_dice={:?}, pos={}, events={:?}",
        state_after_roll1.doubles_count,
        state_after_roll1.last_dice,
        state_after_roll1.find_player(p1_id).unwrap().position,
        events1
    );
    assert_eq!(state_after_roll1.doubles_count, 1);
    assert!(!state_after_roll1.find_player(p1_id).unwrap().in_jail);

    let mut state2 = state_after_roll1;
    state2.turn_phase = TurnPhase::WaitingForRoll;
    let mut rng2 = create_seeded_rng(doubles_seeds[1].0);
    let (state_after_roll2, _) =
        apply_action(&state2, p1_id, &ClientMessage::RollDice, &mut rng2).unwrap();
    assert_eq!(state_after_roll2.doubles_count, 2);
    assert!(!state_after_roll2.find_player(p1_id).unwrap().in_jail);

    let mut state3 = state_after_roll2;
    state3.turn_phase = TurnPhase::WaitingForRoll;
    let mut rng3 = create_seeded_rng(doubles_seeds[2].0);
    let (state_after_roll3, events) =
        apply_action(&state3, p1_id, &ClientMessage::RollDice, &mut rng3).unwrap();
    assert_eq!(state_after_roll3.doubles_count, 0);
    let p1 = state_after_roll3.find_player(p1_id).unwrap();
    assert!(p1.in_jail);
    assert_eq!(p1.position, 10);
    assert_eq!(state_after_roll3.turn_phase, TurnPhase::TurnEnded);

    assert!(events.iter().any(|e| matches!(
        e,
        ServerMessage::DiceRolled {
            player_id,
            new_tile_index: 10,
            is_doubles: true,
            ..
        } if *player_id == p1_id
    )));
}

#[test]
fn test_jail_bail_and_card_mechanics() {
    let (mut state, player_ids) = create_test_game();
    let p1_id = player_ids[0];

    let p1 = state.find_player_mut(p1_id).unwrap();
    p1.in_jail = true;
    p1.position = 10;
    p1.cash = Money::from_vnd(5_000_000);
    p1.get_out_of_jail_cards = 1;

    let mut rng = create_seeded_rng(1);

    let (state_bailed, _) =
        apply_action(&state, p1_id, &ClientMessage::PayJailBail, &mut rng).unwrap();
    let p1_bailed = state_bailed.find_player(p1_id).unwrap();
    assert!(!p1_bailed.in_jail);
    assert_eq!(p1_bailed.cash, Money::from_vnd(4_500_000));

    let (state_card, _) =
        apply_action(&state, p1_id, &ClientMessage::UseJailCard, &mut rng).unwrap();
    let p1_card = state_card.find_player(p1_id).unwrap();
    assert!(!p1_card.in_jail);
    assert_eq!(p1_card.get_out_of_jail_cards, 0);
    assert_eq!(p1_card.cash, Money::from_vnd(5_000_000));
}

#[test]
fn test_bankruptcy_asset_surrender() {
    let (mut state, player_ids) = create_test_game();
    let debtor_id = player_ids[0];
    let creditor_id = player_ids[1];

    state.find_player_mut(debtor_id).unwrap().cash = Money::from_vnd(200_000);
    state.properties[1].owner = Some(debtor_id);

    let mut events = Vec::new();
    handle_bankruptcy(&mut state, debtor_id, Some(creditor_id), &mut events);

    let debtor = state.find_player(debtor_id).unwrap();
    assert!(debtor.is_bankrupt);
    assert_eq!(debtor.cash, Money::ZERO);

    assert_eq!(state.properties[1].owner, Some(creditor_id));

    assert!(events.iter().any(|e| matches!(
        e,
        ServerMessage::PlayerBankrupt { player_id } if *player_id == debtor_id
    )));
}

#[test]
fn test_decline_property_respects_auction_config_option() {
    let (mut state, player_ids) = create_test_game();
    let p1_id = player_ids[0];
    state.find_player_mut(p1_id).unwrap().position = 1;
    state.turn_phase = TurnPhase::TileActionPending;

    let mut rng = create_seeded_rng(7);

    // 1. When enable_auction is true (default), DeclineProperty starts an Auction
    assert!(state.config.enable_auction);
    let (state_with_auction, events_with_auction) =
        apply_action(&state, p1_id, &ClientMessage::DeclineProperty, &mut rng).unwrap();
    assert_eq!(state_with_auction.turn_phase, TurnPhase::Auction);
    assert!(state_with_auction.auction.is_some());
    assert!(events_with_auction
        .iter()
        .any(|e| matches!(e, ServerMessage::AuctionStarted { tile_index: 1, .. })));

    // 2. When ConfigureGame { enable_auction: false } is applied, DeclineProperty skips Auction
    let (state_no_auction_cfg, _) = apply_action(
        &state,
        p1_id,
        &ClientMessage::ConfigureGame {
            enable_auction: false,
            max_players: None,
        },
        &mut rng,
    )
    .unwrap();
    assert!(!state_no_auction_cfg.config.enable_auction);

    let (state_declined, events_declined) = apply_action(
        &state_no_auction_cfg,
        p1_id,
        &ClientMessage::DeclineProperty,
        &mut rng,
    )
    .unwrap();
    assert_eq!(state_declined.turn_phase, TurnPhase::TurnEnded);
    assert!(state_declined.auction.is_none());
    assert!(events_declined.is_empty());
}

#[test]
fn test_build_sell_mortgage_and_rules_engine_queries() {
    let (mut state, _) = create_test_game();
    let p1_id = state.turn_order[0];
    let mut rng = create_seeded_rng(42);

    // Give p1 full District 4 (tiles 1 and 3)
    state.properties[1].owner = Some(p1_id);
    state.properties[3].owner = Some(p1_id);
    state.find_player_mut(p1_id).unwrap().cash = Money::from_vnd(10_000_000);

    let view = state.to_public_view();
    assert!(monopoly_core::rules::RulesEngine::can_build_house(&view, p1_id, 1));
    assert!(!monopoly_core::rules::RulesEngine::can_sell_house(&view, p1_id, 1));
    assert!(monopoly_core::rules::RulesEngine::can_mortgage(&view, p1_id, 1));
    assert_eq!(
        monopoly_core::rules::RulesEngine::tile_rent_preview(&view, 1),
        Money::from_vnd(40_000) // doubled base rent for monopoly
    );

    // Build on tile 1; must build evenly so cannot build 2nd house on tile 1 before tile 3
    let (state, _) = apply_action(
        &state,
        p1_id,
        &ClientMessage::BuildHouse { tile_index: 1 },
        &mut rng,
    )
    .unwrap();
    let view = state.to_public_view();
    assert!(!monopoly_core::rules::RulesEngine::can_build_house(&view, p1_id, 1));
    assert!(monopoly_core::rules::RulesEngine::can_build_house(&view, p1_id, 3));
    assert!(!monopoly_core::rules::RulesEngine::can_mortgage(&view, p1_id, 3));
    assert!(monopoly_core::rules::RulesEngine::can_sell_house(&view, p1_id, 1));

    // Sell house back, then mortgage & unmortgage
    let (state, _) = apply_action(
        &state,
        p1_id,
        &ClientMessage::SellHouse { tile_index: 1 },
        &mut rng,
    )
    .unwrap();
    assert_eq!(state.properties[1].house_count, 0);

    let (state, _) = apply_action(
        &state,
        p1_id,
        &ClientMessage::MortgageProperty { tile_index: 1 },
        &mut rng,
    )
    .unwrap();
    assert!(state.properties[1].is_mortgaged);
    let view = state.to_public_view();
    assert!(monopoly_core::rules::RulesEngine::can_unmortgage(&view, p1_id, 1));

    let (state, _) = apply_action(
        &state,
        p1_id,
        &ClientMessage::UnmortgageProperty { tile_index: 1 },
        &mut rng,
    )
    .unwrap();
    assert!(!state.properties[1].is_mortgaged);
}

#[test]
fn test_bot_policy_and_public_view_omits_secrets() {
    let (mut state, _) = create_test_game();
    let p1_id = state.turn_order[0];
    state.turn_phase = TurnPhase::TileActionPending;
    state.find_player_mut(p1_id).unwrap().position = 1;
    state.find_player_mut(p1_id).unwrap().cash = Money::from_vnd(5_000_000);

    let view = state.to_public_view();
    let serialized = serde_json::to_string(&view).unwrap();
    assert!(!serialized.contains("session_token"));
    assert!(!serialized.contains("chance_deck"));

    let action = monopoly_core::bot::BotPolicy::decide_action(&view, p1_id);
    assert_eq!(action, Some(ClientMessage::BuyProperty));
}
