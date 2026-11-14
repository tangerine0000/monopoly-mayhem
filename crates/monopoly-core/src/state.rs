use crate::board::{BoardCatalog, TileKind};
use crate::cards::{get_chance_deck, get_community_chest_deck, Card};
use monopoly_proto::{
    Money, PlayerId, PublicAuctionView, PublicGameConfig, PublicGameView, PublicPlayerView,
    PublicPropertyView, PublicTurnPhase, SessionToken,
};
use serde::{Deserialize, Serialize};

pub const STARTING_CASH_VND: i64 = 15_000_000;
pub const PASS_GO_REWARD_VND: i64 = 2_000_000;
pub const JAIL_BAIL_VND: i64 = 500_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: PlayerId,
    pub name: String,
    pub seat_index: u8,
    pub cash: Money,
    pub position: u8,
    pub in_jail: bool,
    pub turns_in_jail: u8,
    pub get_out_of_jail_cards: u8,
    pub is_bankrupt: bool,
    pub is_connected: bool,
    pub is_bot_controlled: bool,
    pub session_token: Option<SessionToken>,
}

impl PlayerState {
    pub fn new(
        id: PlayerId,
        name: String,
        seat_index: u8,
        session_token: Option<SessionToken>,
    ) -> Self {
        Self {
            id,
            name,
            seat_index,
            cash: Money::from_vnd(STARTING_CASH_VND),
            position: 0,
            in_jail: false,
            turns_in_jail: 0,
            get_out_of_jail_cards: 0,
            is_bankrupt: false,
            is_connected: true,
            is_bot_controlled: false,
            session_token,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PropertyState {
    pub owner: Option<PlayerId>,
    pub house_count: u8, // 0 = base, 1..4 = houses, 5 = hotel
    pub is_mortgaged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuctionSession {
    pub tile_index: u8,
    pub current_bid: Money,
    pub highest_bidder: Option<PlayerId>,
    pub active_bidders: Vec<PlayerId>,
}

fn default_max_players() -> u8 {
    4
}

fn default_turn_time_limit() -> u32 {
    120
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameConfig {
    pub enable_auction: bool,
    #[serde(default = "default_max_players")]
    pub max_players: u8,
    #[serde(default = "default_turn_time_limit")]
    pub turn_time_limit_seconds: u32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            enable_auction: true,
            max_players: 4,
            turn_time_limit_seconds: 120,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnPhase {
    WaitingForRoll,
    TileActionPending,
    Auction,
    TurnEnded,
}

impl From<TurnPhase> for PublicTurnPhase {
    fn from(value: TurnPhase) -> Self {
        match value {
            TurnPhase::WaitingForRoll => PublicTurnPhase::WaitingForRoll,
            TurnPhase::TileActionPending => PublicTurnPhase::TileActionPending,
            TurnPhase::Auction => PublicTurnPhase::Auction,
            TurnPhase::TurnEnded => PublicTurnPhase::TurnEnded,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameState {
    pub config: GameConfig,
    pub players: Vec<PlayerState>,
    pub turn_order: Vec<PlayerId>,
    pub current_player_index: usize,
    pub properties: Vec<PropertyState>, // 40 elements
    pub turn_phase: TurnPhase,
    pub doubles_count: u8,
    pub last_dice: (u8, u8),
    pub chance_deck: Vec<Card>,
    pub chance_discard: Vec<Card>,
    pub community_chest_deck: Vec<Card>,
    pub community_chest_discard: Vec<Card>,
    pub auction: Option<AuctionSession>,
    pub is_game_started: bool,
    pub is_game_over: bool,
    pub winner: Option<PlayerId>,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    pub fn new() -> Self {
        Self {
            config: GameConfig::default(),
            players: Vec::new(),
            turn_order: Vec::new(),
            current_player_index: 0,
            properties: vec![PropertyState::default(); 40],
            turn_phase: TurnPhase::WaitingForRoll,
            doubles_count: 0,
            last_dice: (1, 1),
            chance_deck: get_chance_deck(),
            chance_discard: Vec::new(),
            community_chest_deck: get_community_chest_deck(),
            community_chest_discard: Vec::new(),
            auction: None,
            is_game_started: false,
            is_game_over: false,
            winner: None,
        }
    }

    pub fn current_player(&self) -> Option<&PlayerState> {
        if self.turn_order.is_empty() {
            return None;
        }
        let current_id = self.turn_order.get(self.current_player_index)?;
        self.find_player(*current_id)
    }

    pub fn current_player_mut(&mut self) -> Option<&mut PlayerState> {
        if self.turn_order.is_empty() {
            return None;
        }
        let current_id = *self.turn_order.get(self.current_player_index)?;
        self.find_player_mut(current_id)
    }

    pub fn find_player(&self, id: PlayerId) -> Option<&PlayerState> {
        self.players.iter().find(|p| p.id == id)
    }

    pub fn find_player_mut(&mut self, id: PlayerId) -> Option<&mut PlayerState> {
        self.players.iter_mut().find(|p| p.id == id)
    }

    pub fn active_players(&self) -> Vec<&PlayerState> {
        self.players.iter().filter(|p| !p.is_bankrupt).collect()
    }

    pub fn owns_full_district(
        &self,
        player_id: PlayerId,
        district: crate::board::District,
    ) -> bool {
        let district_tiles = crate::board::get_district_tiles(district);
        district_tiles.iter().all(|&t| {
            self.properties[t as usize].owner == Some(player_id)
                && !self.properties[t as usize].is_mortgaged
        })
    }

    pub fn stations_owned_by(&self, player_id: PlayerId) -> usize {
        [5usize, 15, 25, 35]
            .iter()
            .filter(|&&idx| {
                self.properties[idx].owner == Some(player_id) && !self.properties[idx].is_mortgaged
            })
            .count()
    }

    pub fn utilities_owned_by(&self, player_id: PlayerId) -> usize {
        [12usize, 28]
            .iter()
            .filter(|&&idx| {
                self.properties[idx].owner == Some(player_id) && !self.properties[idx].is_mortgaged
            })
            .count()
    }

    pub fn is_tile_purchasable(&self, tile_index: u8) -> bool {
        let tile = BoardCatalog::tile(tile_index);
        match &tile.kind {
            TileKind::Property(_) | TileKind::Station(_) | TileKind::Utility(_) => {
                self.properties[tile_index as usize].owner.is_none()
            }
            _ => false,
        }
    }

    pub fn to_public_view(&self) -> PublicGameView {
        PublicGameView {
            config: PublicGameConfig {
                enable_auction: self.config.enable_auction,
                max_players: self.config.max_players,
                turn_time_limit_seconds: self.config.turn_time_limit_seconds,
            },
            players: self
                .players
                .iter()
                .map(|p| PublicPlayerView {
                    id: p.id,
                    name: p.name.clone(),
                    seat_index: p.seat_index,
                    cash: p.cash,
                    position: p.position,
                    in_jail: p.in_jail,
                    turns_in_jail: p.turns_in_jail,
                    get_out_of_jail_cards: p.get_out_of_jail_cards,
                    is_bankrupt: p.is_bankrupt,
                    is_connected: p.is_connected,
                    is_bot_controlled: p.is_bot_controlled,
                })
                .collect(),
            turn_order: self.turn_order.clone(),
            current_player_index: self.current_player_index,
            properties: self
                .properties
                .iter()
                .map(|prop| PublicPropertyView {
                    owner: prop.owner,
                    house_count: prop.house_count,
                    is_mortgaged: prop.is_mortgaged,
                })
                .collect(),
            turn_phase: self.turn_phase.into(),
            doubles_count: self.doubles_count,
            last_dice: self.last_dice,
            auction: self.auction.as_ref().map(|a| PublicAuctionView {
                tile_index: a.tile_index,
                current_bid: a.current_bid,
                highest_bidder: a.highest_bidder,
                active_bidders: a.active_bidders.clone(),
            }),
            is_game_started: self.is_game_started,
            is_game_over: self.is_game_over,
            winner: self.winner,
        }
    }
}
