use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RoomId(pub String);

impl fmt::Display for RoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PlayerId(pub Uuid);

impl fmt::Display for PlayerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SessionToken(pub String);

impl fmt::Display for SessionToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
)]
pub struct Money(pub i64); // 1 unit = 1,000 VNĐ

impl Money {
    pub const ZERO: Money = Money(0);

    pub const fn from_vnd(vnd: i64) -> Self {
        Money(vnd / 1_000)
    }

    pub const fn as_vnd(&self) -> i64 {
        self.0 * 1_000
    }

    pub fn format(&self, locale: Locale) -> String {
        let vnd = self.as_vnd();
        let is_negative = vnd < 0;
        let s = vnd.unsigned_abs().to_string();
        let sep = match locale {
            Locale::Vi => '.',
            Locale::En => ',',
        };
        let suffix = match locale {
            Locale::Vi => " đ",
            Locale::En => " VND",
        };

        let mut formatted = String::with_capacity(s.len() + s.len() / 3);
        let len = s.len();
        for (i, ch) in s.chars().enumerate() {
            if i > 0 && (len - i).is_multiple_of(3) {
                formatted.push(sep);
            }
            formatted.push(ch);
        }

        if is_negative {
            format!("-{}{}", formatted, suffix)
        } else {
            format!("{}{}", formatted, suffix)
        }
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format(Locale::Vi))
    }
}

impl std::ops::Add for Money {
    type Output = Money;
    fn add(self, rhs: Self) -> Self::Output {
        Money(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Money {
    type Output = Money;
    fn sub(self, rhs: Self) -> Self::Output {
        Money(self.0 - rhs.0)
    }
}

impl std::ops::AddAssign for Money {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl std::ops::SubAssign for Money {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum ClientMessage {
    JoinRoom {
        player_name: String,
        session_token: Option<SessionToken>,
    },
    RollDice,
    BuyProperty,
    DeclineProperty,
    PlaceBid {
        amount: Money,
    },
    PassAuction,
    BuildHouse {
        tile_index: u8,
    },
    SellHouse {
        tile_index: u8,
    },
    MortgageProperty {
        tile_index: u8,
    },
    UnmortgageProperty {
        tile_index: u8,
    },
    PayJailBail,
    UseJailCard,
    DeclareBankruptcy,
    EndTurn,
    ConfigureGame {
        enable_auction: bool,
        #[serde(default)]
        max_players: Option<u8>,
    },
    StartGame,
    AddBot,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum Locale {
    #[default]
    Vi,
    En,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DeckId {
    Chance,
    CommunityChest,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TileId(pub u8);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CardId(pub u16);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageKey {
    // Room / Lobby
    RoomAlreadyStarted,
    RoomFull,
    HostOnlyStart,
    HostOnlyAddBot,
    NeedAtLeastTwoPlayers,
    BotName { index: u8 },
    Timeout,
    Disconnected,
    // Game rules
    GameNotStarted,
    GameAlreadyStarted,
    GameAlreadyOver,
    NotYourTurn,
    InvalidPhase,
    InsufficientFunds,
    PropertyNotPurchasable,
    PropertyAlreadyOwned,
    MustOwnFullDistrict,
    PropertyMortgaged,
    MustBuildEvenly,
    MustSellEvenly,
    MaxHousesReached,
    NoHousesToSell,
    CannotMortgageWithBuildings,
    PropertyNotMortgaged,
    NotInJail,
    NoJailCard,
    AuctionNotActive,
    BidTooLow,
    NotEligibleToBid,
    InvalidTileIndex,
    InvalidPlayerCount,
    DebtNotExceeded,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PublicTurnPhase {
    WaitingForRoll,
    TileActionPending,
    Auction,
    TurnEnded,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicGameConfig {
    pub enable_auction: bool,
    pub max_players: u8,
    pub turn_time_limit_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicPlayerView {
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
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PublicPropertyView {
    pub owner: Option<PlayerId>,
    pub house_count: u8,
    pub is_mortgaged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicAuctionView {
    pub tile_index: u8,
    pub current_bid: Money,
    pub highest_bidder: Option<PlayerId>,
    pub active_bidders: Vec<PlayerId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicGameView {
    pub config: PublicGameConfig,
    pub players: Vec<PublicPlayerView>,
    pub turn_order: Vec<PlayerId>,
    pub current_player_index: usize,
    pub properties: Vec<PublicPropertyView>,
    pub turn_phase: PublicTurnPhase,
    pub doubles_count: u8,
    pub last_dice: (u8, u8),
    pub auction: Option<PublicAuctionView>,
    pub is_game_started: bool,
    pub is_game_over: bool,
    pub winner: Option<PlayerId>,
}

impl PublicGameView {
    pub fn current_player(&self) -> Option<&PublicPlayerView> {
        let id = self.turn_order.get(self.current_player_index)?;
        self.players.iter().find(|p| &p.id == id)
    }

    pub fn find_player(&self, id: PlayerId) -> Option<&PublicPlayerView> {
        self.players.iter().find(|p| p.id == id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMessage {
    RoomJoined {
        room_id: RoomId,
        player_id: PlayerId,
        session_token: SessionToken,
        seat_index: u8,
    },
    GameStarted {
        turn_order: Vec<PlayerId>,
    },
    TurnStarted {
        active_player: PlayerId,
        time_limit_seconds: u32,
    },
    TimerTick {
        seconds_remaining: u32,
    },
    DiceRolled {
        player_id: PlayerId,
        dice: (u8, u8),
        new_tile_index: u8,
        is_doubles: bool,
    },
    PropertyAcquired {
        player_id: PlayerId,
        tile_index: u8,
        cost: Money,
    },
    RentPaid {
        payer: PlayerId,
        recipient: PlayerId,
        tile_index: u8,
        amount: Money,
    },
    AuctionStarted {
        tile_index: u8,
        opening_bid: Money,
    },
    BidPlaced {
        bidder: PlayerId,
        amount: Money,
    },
    AuctionConcluded {
        winner: Option<PlayerId>,
        final_price: Money,
    },
    HouseModified {
        tile_index: u8,
        house_count: u8,
    },
    PropertyMortgageChanged {
        tile_index: u8,
        is_mortgaged: bool,
    },
    CardDrawn {
        player_id: PlayerId,
        deck: DeckId,
        card_id: CardId,
    },
    PlayerBankrupt {
        player_id: PlayerId,
    },
    BotAssumedControl {
        player_id: PlayerId,
        reason: MessageKey,
    },
    PlayerReconnected {
        player_id: PlayerId,
    },
    SyncView {
        view: PublicGameView,
    },
    ActionRejected {
        reason: MessageKey,
    },
}
