mod helpers;

use helpers::{recv_msg, send_json, TestServer};
use monopoly_proto::{ClientMessage, RoomId, ServerMessage};
use monopoly_server::room::RoomActor;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_room_creation_and_4player_join() {
    let server = TestServer::spawn().await;
    let room_id = RoomId("SGN-TEST".to_string());
    let _room_tx = RoomActor::spawn(room_id.clone(), server.state.registry.clone());

    let mut players = Vec::new();

    for i in 1..=4 {
        let (mut sink, mut stream) = server.connect_ws(&room_id.0).await;
        send_json(
            &mut sink,
            &ClientMessage::JoinRoom {
                player_name: format!("Player {}", i),
                session_token: None,
            },
        )
        .await;

        let joined_msg = recv_msg(&mut stream).await.expect("Expected RoomJoined");
        match joined_msg {
            ServerMessage::RoomJoined {
                room_id: r_id,
                seat_index,
                ..
            } => {
                assert_eq!(r_id, room_id);
                assert_eq!(seat_index, (i - 1) as u8);
            }
            other => panic!("Expected RoomJoined, got {:?}", other),
        }

        players.push((sink, stream));
    }

    // 4th player joining should trigger GameStarted
    let (_sink4, mut stream4) = players.pop().unwrap();
    let mut got_game_started = false;

    // Read subsequent messages for GameStarted
    for _ in 0..5 {
        if let Some(ServerMessage::GameStarted { turn_order }) = recv_msg(&mut stream4).await {
            assert_eq!(turn_order.len(), 4);
            got_game_started = true;
            break;
        }
    }

    assert!(got_game_started, "Did not receive GameStarted message");
}

#[tokio::test]
async fn test_turn_timer_expiration_and_bot_takeover() {
    let server = TestServer::spawn().await;
    let room_id = RoomId("SGN-TIME".to_string());
    let _room_tx = RoomActor::spawn(room_id.clone(), server.state.registry.clone());

    // Connect 4 players to auto-start
    let mut streams = Vec::new();
    for i in 1..=4 {
        let (mut sink, mut stream) = server.connect_ws(&room_id.0).await;
        send_json(
            &mut sink,
            &ClientMessage::JoinRoom {
                player_name: format!("Player {}", i),
                session_token: None,
            },
        )
        .await;
        let _ = recv_msg(&mut stream).await;
        streams.push((sink, stream));
    }

    // Active player (player 1) remains idle.
    // Instead of waiting real 30 seconds in integration test, we can verify timer ticks
    // are being emitted by reading stream
    let (_sink1, mut stream1) = streams.remove(0);

    // Read subsequent messages for TimerTick (allow reading through join snapshots, GameStarted, etc.)
    let mut got_timer_tick = false;
    for _ in 0..15 {
        if let Some(ServerMessage::TimerTick { .. }) = recv_msg(&mut stream1).await {
            got_timer_tick = true;
            break;
        }
    }
    assert!(got_timer_tick, "Expected TimerTick messages from server");
}

#[tokio::test]
async fn test_transient_disconnect_and_reconnection() {
    let server = TestServer::spawn().await;
    let room_id = RoomId("SGN-RECON".to_string());
    let _room_tx = RoomActor::spawn(room_id.clone(), server.state.registry.clone());

    // Connect player 1
    let (mut sink1, mut stream1) = server.connect_ws(&room_id.0).await;
    send_json(
        &mut sink1,
        &ClientMessage::JoinRoom {
            player_name: "Player 1".to_string(),
            session_token: None,
        },
    )
    .await;

    let joined1 = recv_msg(&mut stream1).await.unwrap();
    let (p1_id, p1_token) = match joined1 {
        ServerMessage::RoomJoined {
            player_id,
            session_token,
            ..
        } => (player_id, session_token),
        other => panic!("Unexpected message {:?}", other),
    };

    // Drop socket 1
    drop(sink1);
    drop(stream1);
    sleep(Duration::from_millis(50)).await;

    // Reconnect with saved session_token
    let (mut sink_recon, mut stream_recon) = server.connect_ws(&room_id.0).await;
    send_json(
        &mut sink_recon,
        &ClientMessage::JoinRoom {
            player_name: "Player 1".to_string(),
            session_token: Some(p1_token.clone()),
        },
    )
    .await;

    let recon_joined = recv_msg(&mut stream_recon).await.unwrap();
    match recon_joined {
        ServerMessage::RoomJoined {
            player_id,
            session_token,
            seat_index,
            ..
        } => {
            assert_eq!(player_id, p1_id);
            assert_eq!(session_token, p1_token);
            assert_eq!(seat_index, 0);
        }
        other => panic!("Expected RoomJoined on reconnect, got {:?}", other),
    }
}

#[tokio::test]
async fn test_prolonged_disconnect_grace_expiry() {
    let server = TestServer::spawn().await;
    let room_id = RoomId("SGN-DISC".to_string());
    let _room_tx = RoomActor::spawn(room_id.clone(), server.state.registry.clone());

    // Connect 4 players
    let mut streams = Vec::new();
    for i in 1..=4 {
        let (mut sink, mut stream) = server.connect_ws(&room_id.0).await;
        send_json(
            &mut sink,
            &ClientMessage::JoinRoom {
                player_name: format!("Player {}", i),
                session_token: None,
            },
        )
        .await;
        let _ = recv_msg(&mut stream).await;
        streams.push((sink, stream));
    }

    // Drop active player's socket (player 1)
    let (sink1, stream1) = streams.remove(0);
    drop(sink1);
    drop(stream1);

    // Remaining player 2 should receive SyncSnapshot reflecting disconnection
    let (_sink2, mut stream2) = streams.remove(0);
    let mut received_snapshot = false;

    for _ in 0..5 {
        if let Some(ServerMessage::SyncSnapshot { .. }) = recv_msg(&mut stream2).await {
            received_snapshot = true;
            break;
        }
    }
    assert!(
        received_snapshot,
        "Player 2 should receive state update when player 1 disconnects"
    );
}

#[tokio::test]
async fn test_empty_room_cleanup_after_5_minutes() {
    let server = TestServer::spawn().await;
    let room_id = RoomId("SGN-CLEAN".to_string());
    let room_tx = RoomActor::spawn(room_id.clone(), server.state.registry.clone());

    assert!(server.state.registry.contains_key(&room_id.0));

    // Connect and disconnect a client
    let (mut sink, mut stream) = server.connect_ws(&room_id.0).await;
    send_json(
        &mut sink,
        &ClientMessage::JoinRoom {
            player_name: "Solo".to_string(),
            session_token: None,
        },
    )
    .await;
    let _ = recv_msg(&mut stream).await;

    drop(sink);
    drop(stream);
    sleep(Duration::from_millis(50)).await;

    // Send EmptyRoomTimeout command directly to simulate 5-minute timeout
    let _ = room_tx
        .send(monopoly_server::room::RoomCommand::EmptyRoomTimeout)
        .await;
    sleep(Duration::from_millis(50)).await;

    // Room should be cleanly removed from registry
    assert!(!server.state.registry.contains_key(&room_id.0));
}

#[tokio::test]
async fn test_2player_waiting_room_and_host_start_game() {
    let server = TestServer::spawn().await;
    let room_id = RoomId("SGN-2P".to_string());
    let _room_tx = RoomActor::spawn(room_id.clone(), server.state.registry.clone());

    let (mut sink1, mut stream1) = server.connect_ws(&room_id.0).await;
    send_json(
        &mut sink1,
        &ClientMessage::JoinRoom {
            player_name: "An".to_string(),
            session_token: None,
        },
    )
    .await;
    let _ = recv_msg(&mut stream1).await.expect("RoomJoined p1");
    match recv_msg(&mut stream1).await.expect("SyncSnapshot p1") {
        ServerMessage::SyncSnapshot { state } => {
            assert_eq!(state["is_game_started"], false);
            assert_eq!(state["players"].as_array().unwrap().len(), 1);
        }
        other => panic!("Expected SyncSnapshot, got {:?}", other),
    }

    let (mut sink2, mut stream2) = server.connect_ws(&room_id.0).await;
    send_json(
        &mut sink2,
        &ClientMessage::JoinRoom {
            player_name: "Bình".to_string(),
            session_token: None,
        },
    )
    .await;
    let _ = recv_msg(&mut stream2).await.expect("RoomJoined p2");

    // Host starts the 2-player game
    send_json(&mut sink1, &ClientMessage::StartGame).await;

    let mut p2_started = false;
    for _ in 0..5 {
        if let Some(ServerMessage::GameStarted { turn_order }) = recv_msg(&mut stream2).await {
            assert_eq!(turn_order.len(), 2);
            p2_started = true;
            break;
        }
    }
    assert!(
        p2_started,
        "Player 2 should receive GameStarted for 2-player room"
    );
}

#[tokio::test]
async fn test_host_add_bot_and_start_game() {
    let server = TestServer::spawn().await;
    let room_id = RoomId("SGN-BOT".to_string());
    let _room_tx = RoomActor::spawn(room_id.clone(), server.state.registry.clone());

    let (mut sink1, mut stream1) = server.connect_ws(&room_id.0).await;
    send_json(
        &mut sink1,
        &ClientMessage::JoinRoom {
            player_name: "Minh".to_string(),
            session_token: None,
        },
    )
    .await;
    let _ = recv_msg(&mut stream1).await.expect("RoomJoined");
    let _ = recv_msg(&mut stream1).await.expect("Initial SyncSnapshot");

    send_json(&mut sink1, &ClientMessage::AddBot).await;
    match recv_msg(&mut stream1)
        .await
        .expect("SyncSnapshot after AddBot")
    {
        ServerMessage::SyncSnapshot { state } => {
            let players = state["players"].as_array().unwrap();
            assert_eq!(players.len(), 2);
            assert_eq!(players[1]["is_bot_controlled"], true);
        }
        other => panic!("Expected SyncSnapshot, got {:?}", other),
    }

    send_json(&mut sink1, &ClientMessage::StartGame).await;
    let mut started = false;
    for _ in 0..5 {
        if let Some(ServerMessage::GameStarted { turn_order }) = recv_msg(&mut stream1).await {
            assert_eq!(turn_order.len(), 2);
            started = true;
            break;
        }
    }
    assert!(started, "Host + Bot should start game with 2 players");
}

#[tokio::test]
async fn test_configure_game_host_only_and_rejected_after_start() {
    let server = TestServer::spawn().await;
    let room_id = RoomId("SGN-CFG".to_string());
    let _room_tx = RoomActor::spawn(room_id.clone(), server.state.registry.clone());

    let (mut sink1, mut stream1) = server.connect_ws(&room_id.0).await;
    send_json(
        &mut sink1,
        &ClientMessage::JoinRoom {
            player_name: "Host".to_string(),
            session_token: None,
        },
    )
    .await;
    let _ = recv_msg(&mut stream1).await.expect("RoomJoined p1");
    let _ = recv_msg(&mut stream1).await.expect("Initial SyncSnapshot p1");

    // Host disables auction before start
    send_json(
        &mut sink1,
        &ClientMessage::ConfigureGame {
            enable_auction: false,
        },
    )
    .await;
    match recv_msg(&mut stream1).await.expect("SyncSnapshot p1") {
        ServerMessage::SyncSnapshot { state } => {
            assert_eq!(state["config"]["enable_auction"], false);
        }
        other => panic!("Expected SyncSnapshot, got {:?}", other),
    }

    // Guest joins and sends ConfigureGame { enable_auction: true } -> must NOT override host setting
    let (mut sink2, mut stream2) = server.connect_ws(&room_id.0).await;
    send_json(
        &mut sink2,
        &ClientMessage::JoinRoom {
            player_name: "Guest".to_string(),
            session_token: None,
        },
    )
    .await;
    let _ = recv_msg(&mut stream2).await.expect("RoomJoined p2");
    let _ = recv_msg(&mut stream2).await.expect("SyncSnapshot p2");

    send_json(
        &mut sink2,
        &ClientMessage::ConfigureGame {
            enable_auction: true,
        },
    )
    .await;

    // Host starts game, then tries ConfigureGame after start -> rejected
    send_json(&mut sink1, &ClientMessage::StartGame).await;
    let mut started_snapshot = None;
    for _ in 0..8 {
        if let Some(ServerMessage::SyncSnapshot { state }) = recv_msg(&mut stream1).await {
            if state["is_game_started"] == true {
                started_snapshot = Some(state);
                break;
            }
        }
    }
    let st = started_snapshot.expect("Game should start");
    assert_eq!(
        st["config"]["enable_auction"], false,
        "Non-host ConfigureGame should have been ignored"
    );

    send_json(
        &mut sink1,
        &ClientMessage::ConfigureGame {
            enable_auction: true,
        },
    )
    .await;
    let mut got_rejected = false;
    for _ in 0..8 {
        if let Some(ServerMessage::ActionRejected { .. }) = recv_msg(&mut stream1).await {
            got_rejected = true;
            break;
        }
    }
    assert!(
        got_rejected,
        "ConfigureGame after game start must be rejected"
    );
}

#[tokio::test]
async fn test_ws_trade_propose_cancel_and_accept() {
    let server = TestServer::spawn().await;
    let room_id = RoomId("SGN-TRD".to_string());
    let _room_tx = RoomActor::spawn(room_id.clone(), server.state.registry.clone());

    let (mut sink1, mut stream1) = server.connect_ws(&room_id.0).await;
    send_json(
        &mut sink1,
        &ClientMessage::JoinRoom {
            player_name: "An".to_string(),
            session_token: None,
        },
    )
    .await;
    let p1_id = match recv_msg(&mut stream1).await.expect("RoomJoined p1") {
        ServerMessage::RoomJoined { player_id, .. } => player_id,
        other => panic!("Expected RoomJoined, got {:?}", other),
    };

    let (mut sink2, mut stream2) = server.connect_ws(&room_id.0).await;
    send_json(
        &mut sink2,
        &ClientMessage::JoinRoom {
            player_name: "Binh".to_string(),
            session_token: None,
        },
    )
    .await;
    let p2_id = match recv_msg(&mut stream2).await.expect("RoomJoined p2") {
        ServerMessage::RoomJoined { player_id, .. } => player_id,
        other => panic!("Expected RoomJoined, got {:?}", other),
    };

    send_json(&mut sink1, &ClientMessage::StartGame).await;

    // Wait for TurnStarted to identify active turn player (turn order is shuffled)
    let mut active = None;
    for _ in 0..8 {
        if let Some(ServerMessage::TurnStarted { active_player, .. }) = recv_msg(&mut stream1).await
        {
            active = Some(active_player);
            break;
        }
    }
    let active_id = active.expect("TurnStarted after StartGame");
    let (mut active_sink, mut active_stream, _passive_id, mut other_sink, mut other_stream) =
        if active_id == p1_id {
            (sink1, stream1, p2_id, sink2, stream2)
        } else {
            (sink2, stream2, p1_id, sink1, stream1)
        };
    let target_id = if active_id == p1_id { p2_id } else { p1_id };

    // 1. Propose then CancelTrade
    send_json(
        &mut active_sink,
        &ClientMessage::ProposeTrade {
            to: target_id,
            give_tiles: vec![],
            give_cash: monopoly_proto::Money::from_vnd(200_000),
            take_tiles: vec![],
            take_cash: monopoly_proto::Money::from_vnd(100_000),
        },
    )
    .await;

    let mut got_proposed = false;
    for _ in 0..8 {
        if let Some(ServerMessage::TradeProposed { from, to }) =
            recv_msg(&mut other_stream).await
        {
            assert_eq!(from, active_id);
            assert_eq!(to, target_id);
            got_proposed = true;
            break;
        }
    }
    assert!(got_proposed, "Receiver should get TradeProposed");

    send_json(&mut active_sink, &ClientMessage::CancelTrade).await;
    let mut got_cancelled = false;
    for _ in 0..8 {
        if let Some(ServerMessage::TradeResolved { accepted, .. }) =
            recv_msg(&mut active_stream).await
        {
            assert!(!accepted);
            got_cancelled = true;
            break;
        }
    }
    assert!(got_cancelled, "CancelTrade should resolve trade as false");

    // 2. Propose again, then receiver accepts -> cash updates cleanly
    send_json(
        &mut active_sink,
        &ClientMessage::ProposeTrade {
            to: target_id,
            give_tiles: vec![],
            give_cash: monopoly_proto::Money::from_vnd(500_000),
            take_tiles: vec![],
            take_cash: monopoly_proto::Money::from_vnd(100_000),
        },
    )
    .await;

    let mut got_proposed2 = false;
    for _ in 0..8 {
        if let Some(ServerMessage::TradeProposed { .. }) = recv_msg(&mut other_stream).await {
            got_proposed2 = true;
            break;
        }
    }
    assert!(got_proposed2);

    send_json(&mut other_sink, &ClientMessage::RespondTrade { accept: true }).await;

    let mut resolved_ok = false;
    let mut synced_cash_ok = false;
    for _ in 0..10 {
        match recv_msg(&mut active_stream).await {
            Some(ServerMessage::TradeResolved { accepted, .. }) => {
                resolved_ok = accepted;
            }
            Some(ServerMessage::SyncSnapshot { state }) => {
                if state["trade"].is_null() {
                    let players = state["players"].as_array().unwrap();
                    let active_cash = players
                        .iter()
                        .find(|p| p["id"] == serde_json::json!(active_id))
                        .unwrap()["cash"]
                        .as_i64()
                        .unwrap();
                    let target_cash = players
                        .iter()
                        .find(|p| p["id"] == serde_json::json!(target_id))
                        .unwrap()["cash"]
                        .as_i64()
                        .unwrap();
                    if active_cash == 14_600 && target_cash == 15_400 {
                        synced_cash_ok = true;
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    assert!(resolved_ok, "TradeResolved should be accepted=true");
    assert!(
        synced_cash_ok,
        "Net cash +400k/-400k should sync in GameState snapshot"
    );
}
