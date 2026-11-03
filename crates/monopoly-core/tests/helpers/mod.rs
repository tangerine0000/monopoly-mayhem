use monopoly_core::state::{GameState, PlayerState, TurnPhase};
use monopoly_proto::{PlayerId, SessionToken};
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use uuid::Uuid;

pub fn create_test_game() -> (GameState, Vec<PlayerId>) {
    let mut state = GameState::new();
    let id1 = PlayerId(Uuid::from_u128(1));
    let id2 = PlayerId(Uuid::from_u128(2));
    let id3 = PlayerId(Uuid::from_u128(3));
    let id4 = PlayerId(Uuid::from_u128(4));

    let p1 = PlayerState::new(
        id1,
        "Người Chơi 1".to_string(),
        0,
        Some(SessionToken("token1".to_string())),
    );
    let p2 = PlayerState::new(
        id2,
        "Người Chơi 2".to_string(),
        1,
        Some(SessionToken("token2".to_string())),
    );
    let p3 = PlayerState::new(
        id3,
        "Người Chơi 3".to_string(),
        2,
        Some(SessionToken("token3".to_string())),
    );
    let p4 = PlayerState::new(
        id4,
        "Người Chơi 4".to_string(),
        3,
        Some(SessionToken("token4".to_string())),
    );

    state.players = vec![p1, p2, p3, p4];
    state.turn_order = vec![id1, id2, id3, id4];
    state.current_player_index = 0;
    state.is_game_started = true;
    state.turn_phase = TurnPhase::WaitingForRoll;

    (state, vec![id1, id2, id3, id4])
}

pub fn create_seeded_rng(seed: u64) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(seed)
}
