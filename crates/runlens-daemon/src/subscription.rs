use runlens_core::event_v2::EventV2;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(pub u64);

pub struct SubscriptionManager {
    inner: Arc<Mutex<Inner>>,
    next_id: AtomicU64,
}

struct Sub {
    session_id: Option<String>,
    sender: broadcast::Sender<EventV2>,
}

struct Inner {
    subs: HashMap<SubscriptionId, Sub>,
}

impl SubscriptionManager {
    const CHANNEL_CAPACITY: usize = 4096;

    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner { subs: HashMap::new() })),
            next_id: AtomicU64::new(1),
        }
    }

    pub async fn subscribe(
        &self,
        session_id: Option<String>,
    ) -> (SubscriptionId, broadcast::Receiver<EventV2>) {
        let id = SubscriptionId(self.next_id.fetch_add(1, Ordering::SeqCst));
        let (sender, rx) = broadcast::channel(Self::CHANNEL_CAPACITY);
        let mut inner = self.inner.lock().await;
        inner.subs.insert(id, Sub { session_id, sender });
        (id, rx)
    }

    pub async fn unsubscribe(&self, id: SubscriptionId) {
        let mut inner = self.inner.lock().await;
        inner.subs.remove(&id);
    }

    pub async fn publish(&self, event: &EventV2) {
        let inner = self.inner.lock().await;
        for sub in inner.subs.values() {
            if sub.session_id.as_deref().is_some_and(|sid| sid != event.session_id) {
                continue;
            }
            let _ = sub.sender.send(event.clone());
        }
    }

    pub async fn active_count(&self) -> usize {
        self.inner.lock().await.subs.len()
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runlens_core::identifier::Identifier;
    use runlens_core::model::{EventSource, PrivacyClassification, Severity};
    use tokio::time::{timeout, Duration};

    fn dummy_event(session_id: &str) -> EventV2 {
        EventV2::new(
            Identifier::now(),
            Identifier::from_string(session_id).unwrap(),
            Identifier::now(),
            0,
            EventSource::Cli,
            "test",
            Severity::Info,
            serde_json::json!({}),
            PrivacyClassification::Internal,
        )
    }

    #[tokio::test]
    async fn subscribe_and_receive_matching() {
        let mgr = SubscriptionManager::new();
        let sid = Identifier::now().to_string();
        let (_id, mut rx) = mgr.subscribe(Some(sid.clone())).await;
        let ev = dummy_event(&sid);
        mgr.publish(&ev).await;
        let got = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        assert_eq!(got.event_id, ev.event_id);
    }

    #[tokio::test]
    async fn global_subscription_receives_all() {
        let mgr = SubscriptionManager::new();
        let (_id, mut rx) = mgr.subscribe(None).await;
        let sid_a = Identifier::now().to_string();
        let sid_b = Identifier::now().to_string();
        mgr.publish(&dummy_event(&sid_a)).await;
        mgr.publish(&dummy_event(&sid_b)).await;
        let got1 = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        let got2 = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        assert_ne!(got1.event_id, got2.event_id);
    }

    #[tokio::test]
    async fn subscription_filters_by_session() {
        let mgr = SubscriptionManager::new();
        let sid_a = Identifier::now().to_string();
        let sid_b = Identifier::now().to_string();
        let (_id, mut rx) = mgr.subscribe(Some(sid_a.clone())).await;
        mgr.publish(&dummy_event(&sid_b)).await;
        let outcome = timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(outcome.is_err(), "expected timeout (no matching event)");
    }

    #[tokio::test]
    async fn unsubscribe_stops_delivery() {
        let mgr = SubscriptionManager::new();
        let sid = Identifier::now().to_string();
        let (id, mut rx) = mgr.subscribe(Some(sid.clone())).await;
        mgr.unsubscribe(id).await;
        let outcome = rx.recv().await;
        assert!(outcome.is_err(), "expected channel closed after unsubscribe");
    }

    #[tokio::test]
    async fn active_count_tracks_subscriptions() {
        let mgr = SubscriptionManager::new();
        assert_eq!(mgr.active_count().await, 0);
        let (id, _) = mgr.subscribe(Some("s1".into())).await;
        assert_eq!(mgr.active_count().await, 1);
        let (_id2, _) = mgr.subscribe(None).await;
        assert_eq!(mgr.active_count().await, 2);
        mgr.unsubscribe(id).await;
        assert_eq!(mgr.active_count().await, 1);
    }
}
