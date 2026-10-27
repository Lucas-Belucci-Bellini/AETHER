//! Events flow across modules; background e-mails retry and drain on shutdown.
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use liga::prelude::StatusCode;
use liga::web::testing::TestClient;
use modular_api::{Account, AuditEntry, Outbox, SentEmail, app_builder};
use serde_json::json;

async fn app() -> liga::App {
    app_builder()
        .env_vars([("MODULAR__NOTIFICATIONS__RETRY_BASE_MS", "5")])
        .build()
        .await
        .unwrap()
}

async fn wait_for_emails(client: &TestClient, count: usize) -> Vec<SentEmail> {
    for _ in 0..200 {
        let sent: Vec<SentEmail> = client.get("/notifications").await.json();
        if sent.len() >= count {
            return sent;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("e-mails were not sent in time");
}

#[tokio::test]
async fn modules_start_dependencies_first() {
    let app = app().await;
    assert_eq!(app.module_names(), ["accounts", "notifications", "audit"]);
    app.shutdown().await.unwrap();
}

#[tokio::test]
async fn creating_an_account_reaches_every_module() {
    let app = app().await;
    let client = app.test_client();

    let res = client
        .post_json("/accounts", &json!({"email": "  Ana@Example.com "}))
        .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let account: Account = res.json();
    assert_eq!(account.email, "ana@example.com");

    // Event handlers ran before the response was sent.
    let audit: Vec<AuditEntry> = client.get("/audit").await.json();
    assert_eq!(
        audit,
        [AuditEntry {
            action: "account.created".into(),
            account_id: account.id
        }]
    );

    // The e-mail is sent in the background after one transient failure.
    let sent = wait_for_emails(&client, 1).await;
    assert_eq!(
        sent,
        [SentEmail {
            account_id: account.id,
            template: "welcome".into(),
            attempts: 2
        }]
    );
    app.shutdown().await.unwrap();
}

#[tokio::test]
async fn invalid_accounts_are_rejected_without_side_effects() {
    let app = app().await;
    let client = app.test_client();
    let res = client
        .post_json("/accounts", &json!({"email": "not-an-email"}))
        .await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        client
            .get("/audit")
            .await
            .json::<Vec<AuditEntry>>()
            .is_empty()
    );
    app.shutdown().await.unwrap();
}

#[tokio::test]
async fn shutdown_drains_queued_emails() {
    let app = app().await;
    let client = app.test_client();
    for i in 0..5 {
        let res = client
            .post_json(
                "/accounts",
                &json!({"email": format!("user{i}@example.com")}),
            )
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }
    let outbox = app.context().require::<Outbox>().unwrap();
    app.shutdown().await.unwrap();
    assert_eq!(outbox.list().len(), 5);
}
