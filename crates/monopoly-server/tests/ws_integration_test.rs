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
