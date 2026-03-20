// Handler für SSE-Events.

use axum::{
    extract::{Path, State},
    response::{sse::Event, Sse},
};
use futures::{stream, Stream};
use tokio::sync::broadcast;

use crate::state::AppState;

/// GET /api/projects/:id/events – Server-Sent Events Stream für ein Projekt.
pub async fn project_events(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut events = state.events.lock().await;
    let tx = events
        .entry(id.clone())
        .or_insert_with(|| broadcast::channel::<String>(100).0)
        .clone();
    let rx = tx.subscribe();
    drop(events);

    let out = stream::unfold(rx, move |mut rx| async move {
        match rx.recv().await {
            Ok(msg) => Some((Ok(Event::default().event("project_event").data(msg)), rx)),
            Err(broadcast::error::RecvError::Lagged(_)) => {
                Some((Ok(Event::default().event("heartbeat").data("ping")), rx))
            }
            Err(broadcast::error::RecvError::Closed) => None,
        }
    });
    Sse::new(out)
}
