use crate::board::{get_board, TileKind};
use crate::cards::{get_chance_deck, get_community_chest_deck, Card};
use crate::money::Money;
use monopoly_proto::{PlayerId, SessionToken};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameConfig {
    pub enable_auction: bool,
    #[serde(default = "default_max_players")]
    pub max_players: u8,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            enable_auction: true,
            max_players: 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnPhase {
    WaitingForRoll,
    TileActionPending,
    Auction,
    TurnEnded,
    /// Người chơi đang nợ và phải gom tiền (bán nhà / thế chấp) hoặc tuyên bố phá sản.
    Debt,
}

/// Khoản nợ đang treo. `creditors` rỗng nghĩa là nợ ngân hàng; nhiều người thì chia đều.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebtSession {
    pub debtor: PlayerId,
    pub creditors: Vec<PlayerId>,
    pub amount: Money,
    pub tile_index: Option<u8>,
}

/// Đề nghị giao dịch giữa người đang tới lượt (`from`) và người nhận (`to`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeOffer {
    pub from: PlayerId,
    pub to: PlayerId,
    pub give_tiles: Vec<u8>,
    pub give_cash: Money,
    pub take_tiles: Vec<u8>,
    pub take_cash: Money,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    #[serde(default)]
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
    #[serde(default)]
    pub debt: Option<DebtSession>,
    #[serde(default)]
    pub trade: Option<TradeOffer>,
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
            debt: None,
            trade: None,
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

    /// Số tiền tối đa người chơi gom được nếu bán hết nhà (nửa giá) và thế chấp mọi ô.
    pub fn liquidation_value(&self, player_id: PlayerId) -> Money {
        let board = get_board();
        let mut total = Money::ZERO;
        for (idx, prop) in self.properties.iter().enumerate() {
            if prop.owner != Some(player_id) {
                continue;
            }
            match &board[idx].kind {
                TileKind::Property(p) => {
                    total += Money(p.house_cost.0 / 2 * prop.house_count as i64);
                    if !prop.is_mortgaged {
                        total += p.mortgage_value;
                    }
                }
                TileKind::Station(st) if !prop.is_mortgaged => total += st.mortgage_value,
                TileKind::Utility(u) if !prop.is_mortgaged => total += u.mortgage_value,
                _ => {}
            }
        }
        total
    }

    /// Tổng tài sản để xếp hạng: tiền mặt + giá đất (nửa nếu thế chấp) + giá nhà.
    pub fn net_worth(&self, player_id: PlayerId) -> Money {
        let board = get_board();
        let mut total = self.find_player(player_id).map(|p| p.cash).unwrap_or(Money::ZERO);
        for (idx, prop) in self.properties.iter().enumerate() {
            if prop.owner != Some(player_id) {
                continue;
            }
            let (cost, house_cost) = match &board[idx].kind {
                TileKind::Property(p) => (p.cost, p.house_cost),
                TileKind::Station(st) => (st.cost, Money::ZERO),
                TileKind::Utility(u) => (u.cost, Money::ZERO),
                _ => continue,
            };
            total += if prop.is_mortgaged { Money(cost.0 / 2) } else { cost };
            total += Money(house_cost.0 * prop.house_count as i64);
        }
        total
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
        let board = get_board();
        let tile = &board[tile_index as usize];
        match &tile.kind {
            TileKind::Property(_) | TileKind::Station(_) | TileKind::Utility(_) => {
                self.properties[tile_index as usize].owner.is_none()
            }
            _ => false,
        }
    }
}
