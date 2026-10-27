//! Integration tests for the catalog API (no network port is opened).
#![allow(clippy::unwrap_used)]

use hello_api::{Product, app_builder};
use liga::prelude::StatusCode;
use serde_json::{Value, json};

async fn app(env: &[(&str, &str)]) -> liga::App {
    let mut vars = vec![("HELLO__CATALOG__SEED_DEMO_DATA", "false")];
    vars.extend_from_slice(env);
    app_builder().env_vars(vars).build().await.unwrap()
}

#[tokio::test]
async fn health_and_seeded_products() {
    let app = app_builder()
        .env_vars(Vec::<(String, String)>::new())
        .build()
        .await
        .unwrap();
    let client = app.test_client();
    assert_eq!(client.get("/health").await.json::<Value>()["status"], "ok");
    let products: Vec<Product> = client.get("/products").await.json();
    assert_eq!(products.len(), 2);
    app.shutdown().await.unwrap();
}

#[tokio::test]
async fn create_then_fetch_a_product() {
    let app = app(&[]).await;
    let client = app.test_client();

    let res = client
        .post_json(
            "/products",
            &json!({"name": "  Keyboard ", "price_cents": 12_900}),
        )
        .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let created: Product = res.json();
    assert_eq!(created.name, "Keyboard");

    let fetched: Product = client
        .get(&format!("/products/{}", created.id))
        .await
        .json();
    assert_eq!(fetched, created);
    assert_eq!(
        client.get("/products").await.json::<Vec<Product>>(),
        [created]
    );
}

#[tokio::test]
async fn errors_are_consistent() {
    let app = app(&[]).await;
    let client = app.test_client();

    let res = client.get("/products/999").await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body: Value = res.json();
    assert_eq!(body["error"]["code"], "not_found");
    assert_eq!(body["error"]["message"], "product 999 not found");
    assert!(body["error"]["request_id"].is_string());

    let res = client.get("/products/abc").await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    let res = client
        .post_json("/products", &json!({"name": "", "price_cents": 0}))
        .await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let fields: Vec<String> = res.json::<Value>()["error"]["details"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["field"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(fields, ["name", "price_cents"]);

    let res = client
        .post_json(
            "/products",
            &json!({"name": "x", "price_cents": 1, "admin": true}),
        )
        .await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res.json::<Value>()["error"]["code"], "invalid_body");
}

#[tokio::test]
async fn full_catalog_is_a_conflict() {
    let app = app(&[("HELLO__CATALOG__MAX_PRODUCTS", "1")]).await;
    let client = app.test_client();
    let new = json!({"name": "only one", "price_cents": 100});
    assert_eq!(
        client.post_json("/products", &new).await.status(),
        StatusCode::CREATED
    );
    let res = client.post_json("/products", &new).await;
    assert_eq!(res.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn invalid_configuration_is_rejected_at_startup() {
    let err = app_builder()
        .env_vars([("HELLO__CATALOG__MAX_PRODUCTS", "0")])
        .build()
        .await
        .unwrap_err();
    assert!(err.to_string().contains("max_products"), "{err}");

    let err = app_builder()
        .env_vars([("HELLO__CATALOG__MAX_PRODCTS", "5")])
        .build()
        .await
        .unwrap_err();
    assert!(err.to_string().contains("unknown field"), "{err}");
}
