use monopoly_proto::PlayerId;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;

#[derive(Debug)]
pub enum TimerNotification {
    Tick(u32),
    TurnExpired(PlayerId),
    GraceExpired(PlayerId),
}

pub struct TurnTimer;

impl TurnTimer {
    pub fn spawn_turn_countdown(
        player_id: PlayerId,
        seconds: u32,
        notify_tx: mpsc::Sender<TimerNotification>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            for s in (0..=seconds).rev() {
                if notify_tx.send(TimerNotification::Tick(s)).await.is_err() {
                    return;
                }
                if s == 0 {
                    let _ = notify_tx
                        .send(TimerNotification::TurnExpired(player_id))
                        .await;
                    break;
                }
                sleep(Duration::from_secs(1)).await;
            }
        })
    }

    pub fn spawn_disconnect_grace(
        player_id: PlayerId,
        grace_seconds: u32,
        notify_tx: mpsc::Sender<TimerNotification>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            sleep(Duration::from_secs(grace_seconds as u64)).await;
            let _ = notify_tx
                .send(TimerNotification::GraceExpired(player_id))
                .await;
        })
    }
}
