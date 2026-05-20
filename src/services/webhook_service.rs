// Outgoing Webhook Service: sendet Events an konfigurierte Webhook-URLs.
//
// Unterstützte Events: task.created, task.moved, task.approved,
// task.rejected, task.commented
//
// Retry-Strategie: bis zu 3 Versuche mit linearem Backoff (1s, 2s).

use serde::{Deserialize, Serialize};

/// Payload eines ausgehenden Webhooks.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebhookEvent {
    /// Event-Typ, z.B. "task.moved"
    pub event: String,
    /// Projekt-Slug
    pub project: String,
    /// Task-Informationen
    pub task: WebhookTaskInfo,
    /// ISO-8601-Zeitstempel
    pub ts: String,
}

/// Task-Informationen im Webhook-Payload.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebhookTaskInfo {
    pub id: String,
    pub title: String,
    /// Name der Zielspalte
    pub column: String,
    /// Zugewiesener Bearbeiter
    pub worker: String,
}

/// Sendet einen Webhook-Event an die angegebene URL.
/// Diese Funktion ist nicht async-blockierend gegenüber dem Aufrufer –
/// sie wird typischerweise via `tokio::spawn` im Hintergrund aufgerufen.
///
/// Retry: 3 Versuche, linearer Backoff 1s / 2s.
pub async fn fire_webhook(client: &reqwest::Client, url: &str, event: &WebhookEvent) {
    for attempt in 0..3u32 {
        if attempt > 0 {
            tokio::time::sleep(tokio::time::Duration::from_secs(attempt as u64)).await;
        }
        match client
            .post(url)
            .json(event)
            .timeout(tokio::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                tracing::debug!(
                    "Webhook gesendet an {} (event={}, attempt={})",
                    url,
                    event.event,
                    attempt + 1
                );
                return;
            }
            Ok(resp) => {
                tracing::warn!(
                    "Webhook-Fehler an {} (status={}, attempt={})",
                    url,
                    resp.status(),
                    attempt + 1
                );
            }
            Err(e) => {
                tracing::warn!(
                    "Webhook-Netzwerkfehler an {} (attempt={}): {}",
                    url,
                    attempt + 1,
                    e
                );
            }
        }
    }
    tracing::error!(
        "Webhook endgültig fehlgeschlagen nach 3 Versuchen an {}",
        url
    );
}

/// Sendet einen Webhook fire-and-forget im Hintergrund (tokio::spawn).
/// Tut nichts wenn webhook_url None ist.
pub fn dispatch_webhook(client: reqwest::Client, webhook_url: Option<String>, event: WebhookEvent) {
    if let Some(url) = webhook_url {
        tokio::spawn(async move {
            fire_webhook(&client, &url, &event).await;
        });
    }
}
