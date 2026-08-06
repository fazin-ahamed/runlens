use crate::subscription::SubscriptionManager;
use runlens_core::event_v2::EventV2;
use runlens_storage::repo::Repository;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

const EVENTS_PER_COMMIT: usize = 100;
const COMMIT_WINDOW_MS: u64 = 50;

#[derive(Clone)]
pub struct IngestHandle {
    sender: mpsc::Sender<EventV2>,
}

impl IngestHandle {
    pub async fn ingest(&self, event: EventV2) -> Result<(), EventV2> {
        self.sender.send(event).await.map_err(|e| e.0)
    }

    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

pub fn start_ingest_worker(repo: Repository, subscriptions: Arc<SubscriptionManager>) -> IngestHandle {
    let (sender, rx) = mpsc::channel::<EventV2>(10_000);
    let handle = IngestHandle { sender: sender.clone() };

    tokio::spawn(ingest_worker(rx, repo, subscriptions));

    handle
}

async fn ingest_worker(mut rx: mpsc::Receiver<EventV2>, repo: Repository, subscriptions: Arc<SubscriptionManager>) {
    let mut batch = Vec::with_capacity(EVENTS_PER_COMMIT);
    let mut commit_timer = interval(Duration::from_millis(COMMIT_WINDOW_MS));

    // flush on volume, or on a quiet timer so sparse events still land
    loop {
        tokio::select! {
            Some(event) = rx.recv() => {
                batch.push(event);
                if batch.len() >= EVENTS_PER_COMMIT {
                    commit_events(&mut batch, &repo, &subscriptions).await;
                }
            }
            _ = commit_timer.tick() => {
                if !batch.is_empty() {
                    commit_events(&mut batch, &repo, &subscriptions).await;
                }
            }
            else => break,
        }
    }

    while let Some(event) = rx.recv().await {
        batch.push(event);
    }
    if !batch.is_empty() {
        commit_events(&mut batch, &repo, &subscriptions).await;
    }
}

async fn commit_events(batch: &mut Vec<EventV2>, repo: &Repository, subscriptions: &Arc<SubscriptionManager>) {
    if batch.is_empty() {
        return;
    }

    for ev in batch.iter() {
        subscriptions.publish(ev).await;
    }

    let v1_events: Vec<runlens_core::model::Event> = batch.drain(..).map(|ev2| ev2.into()).collect();
    let repo = repo.clone();
    if let Err(e) = tokio::task::spawn_blocking(move || repo.batch_append_events(&v1_events))
        .await
        .expect("blocking task panicked")
    {
        tracing::error!(?e, "batch insert failed");
    }
}
