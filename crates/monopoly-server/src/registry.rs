use crate::room::{RoomActor, RoomCommand};
use dashmap::DashMap;
use monopoly_proto::RoomId;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct LobbyRegistry {
    rooms: Arc<DashMap<String, mpsc::Sender<RoomCommand>>>,
}

impl LobbyRegistry {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(DashMap::new()),
        }
    }

    pub fn create_room(&self) -> RoomId {
        for attempt in 0..16 {
            let suffix_len = if attempt < 8 { 4 } else { 6 };
            let raw_uuid = Uuid::new_v4().simple().to_string();
            let code = format!("SGN-{}", raw_uuid[..suffix_len].to_uppercase());
            if !self.rooms.contains_key(&code) {
                let room_id = RoomId(code);
                RoomActor::spawn(room_id.clone(), self.clone());
                return room_id;
            }
        }
        let fallback = format!("SGN-{}", Uuid::new_v4().simple().to_string()[..8].to_uppercase());
        let room_id = RoomId(fallback);
        RoomActor::spawn(room_id.clone(), self.clone());
        room_id
    }

    pub fn get_or_spawn(&self, code: &str) -> mpsc::Sender<RoomCommand> {
        if let Some(tx) = self.rooms.get(code) {
            return tx.clone();
        }
        RoomActor::spawn(RoomId(code.to_string()), self.clone())
    }

    pub fn insert(&self, code: String, tx: mpsc::Sender<RoomCommand>) {
        self.rooms.insert(code, tx);
    }

    pub fn get(&self, code: &str) -> Option<mpsc::Sender<RoomCommand>> {
        self.rooms.get(code).map(|r| r.clone())
    }

    pub fn remove(&self, code: &str) {
        self.rooms.remove(code);
    }

    pub fn contains(&self, code: &str) -> bool {
        self.rooms.contains_key(code)
    }

    pub fn len(&self) -> usize {
        self.rooms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rooms.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_room_produces_unique_codes() {
        let registry = LobbyRegistry::new();
        let r1 = registry.create_room();
        let r2 = registry.create_room();
        assert_ne!(r1, r2);
        assert!(registry.contains(&r1.0));
        assert!(registry.contains(&r2.0));
    }
}
