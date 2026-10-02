use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use inputkey_core_abstractions::{CoreEvent, EventPublisher, EventSubscriber};

type Handler = Arc<dyn Fn(CoreEvent) + Send + Sync>;

#[derive(Default)]
struct Inner {
    next_id: u64,
    handlers: HashMap<u64, Handler>,
}

#[derive(Clone, Default)]
pub struct EventBus {
    inner: Arc<RwLock<Inner>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }
}

impl EventPublisher for EventBus {
    fn publish(&self, event: CoreEvent) {
        let handlers: Vec<Handler> = self
            .inner
            .read()
            .expect("event bus read lock")
            .handlers
            .values()
            .cloned()
            .collect();
        for handler in handlers {
            handler(event.clone());
        }
    }
}

impl EventSubscriber for EventBus {
    fn subscribe(&self, handler: Box<dyn Fn(CoreEvent) + Send + Sync>) -> Box<dyn FnOnce()> {
        let handler: Handler = Arc::from(handler);
        let id = {
            let mut inner = self.inner.write().expect("event bus write lock");
            inner.next_id += 1;
            let id = inner.next_id;
            inner.handlers.insert(id, handler);
            id
        };
        let inner = Arc::clone(&self.inner);
        Box::new(move || {
            inner
                .write()
                .expect("event bus unsubscribe lock")
                .handlers
                .remove(&id);
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use inputkey_core_abstractions::{CoreEventType, EngineState, RootPhase};

    use super::*;

    fn event(text: &str) -> CoreEvent {
        CoreEvent {
            kind: CoreEventType::StateChanged,
            sequence: 1,
            text: text.to_owned(),
            state: EngineState {
                language_id: "vi".into(),
                phase: RootPhase::Idle,
                raw: String::new(),
                rendered: String::new(),
                child_mode: "start".into(),
                child_phase: "start".into(),
            },
        }
    }

    #[test]
    fn unsubscribe_stops_observation() {
        let bus = EventBus::new();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&seen);
        let unsubscribe = bus.subscribe(Box::new(move |event| {
            sink.lock().expect("sink lock").push(event.text);
        }));
        bus.publish(event("one"));
        unsubscribe();
        bus.publish(event("two"));
        assert_eq!(&*seen.lock().expect("sink lock"), &["one"]);
    }
}
