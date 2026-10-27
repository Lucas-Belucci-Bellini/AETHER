//! A small product catalog served by Liga.
//!
//! Routes:
//! - `GET  /health`
//! - `GET  /products`
//! - `POST /products` (validated JSON body)
//! - `GET  /products/{id}`
//!
//! Configuration (`hello-api.toml` or `HELLO__...` environment variables):
//!
//! ```toml
//! [server]
//! addr = "127.0.0.1:3000"
//!
//! [catalog]
//! max_products = 1000
//! seed_demo_data = true
//! ```

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{PoisonError, RwLock};

use liga::prelude::*;
use serde::{Deserialize, Serialize};

/// The `[catalog]` configuration section.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CatalogConfig {
    /// Upper bound on stored products.
    pub max_products: usize,
    /// Insert two demo products at startup.
    pub seed_demo_data: bool,
}

impl Default for CatalogConfig {
    fn default() -> Self {
        Self {
            max_products: 1_000,
            seed_demo_data: true,
        }
    }
}

impl Validate for CatalogConfig {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();
        errors.check(
            (1..=100_000).contains(&self.max_products),
            "max_products",
            "range",
            "must be between 1 and 100000",
        );
        errors.into_result()
    }
}

/// A product as returned by the API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Product {
    /// Identifier.
    pub id: u64,
    /// Display name.
    pub name: String,
    /// Price in cents (avoids floating point money).
    pub price_cents: u64,
}

/// Body of `POST /products`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewProduct {
    /// Display name, 1–120 characters.
    pub name: String,
    /// Price in cents, 1–100 000 000.
    pub price_cents: u64,
}

impl Validate for NewProduct {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();
        let name = self.name.trim();
        errors
            .check(
                !name.is_empty(),
                "name",
                "required",
                "name must not be empty",
            )
            .check(
                name.chars().count() <= 120,
                "name",
                "length",
                "name must have at most 120 characters",
            )
            .check(
                (1..=100_000_000).contains(&self.price_cents),
                "price_cents",
                "range",
                "price_cents must be between 1 and 100000000",
            );
        errors.into_result()
    }
}

/// Errors of the in-memory store.
#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    /// `max_products` reached.
    #[error("the catalog is full ({0} products)")]
    Full(usize),
}

/// In-memory product store shared through the application state.
#[derive(Debug)]
pub struct Catalog {
    products: RwLock<BTreeMap<u64, Product>>,
    next_id: AtomicU64,
    max_products: usize,
}

impl Catalog {
    /// Empty catalog holding at most `max_products`.
    pub fn new(max_products: usize) -> Self {
        Self {
            products: RwLock::new(BTreeMap::new()),
            next_id: AtomicU64::new(1),
            max_products,
        }
    }

    /// All products ordered by id.
    pub fn list(&self) -> Vec<Product> {
        self.products
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .cloned()
            .collect()
    }

    /// One product.
    pub fn get(&self, id: u64) -> Option<Product> {
        self.products
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&id)
            .cloned()
    }

    /// Stores a validated product.
    pub fn create(&self, new: NewProduct) -> Result<Product, CatalogError> {
        let mut products = self
            .products
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        if products.len() >= self.max_products {
            return Err(CatalogError::Full(self.max_products));
        }
        let product = Product {
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
            name: new.name.trim().to_owned(),
            price_cents: new.price_cents,
        };
        products.insert(product.id, product.clone());
        Ok(product)
    }
}

/// Registers the catalog store and its routes.
#[derive(Debug, Clone, Copy)]
pub struct CatalogModule;

impl Module for CatalogModule {
    fn name(&self) -> &'static str {
        "catalog"
    }

    fn register(&self, ctx: &mut ModuleContext<'_>) -> Result<(), ModuleError> {
        let config: CatalogConfig = ctx.config_section("catalog")?;
        let catalog = Catalog::new(config.max_products);
        if config.seed_demo_data {
            for (name, price_cents) in [("Rust mug", 2_500), ("Ferris plush", 4_990)] {
                catalog
                    .create(NewProduct {
                        name: name.into(),
                        price_cents,
                    })
                    .map_err(|e| ModuleError::with_source("could not seed demo data", e))?;
            }
        }
        ctx.provide(catalog)?;
        ctx.contribute(routes());
        Ok(())
    }
}

fn routes() -> Routes {
    Routes::new()
        .route("/health", get(health))
        .route("/products", get(list_products).post(create_product))
        .route("/products/{id}", get(show_product))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn list_products(catalog: Dep<Catalog>) -> Json<Vec<Product>> {
    Json(catalog.list())
}

async fn show_product(catalog: Dep<Catalog>, Path(id): Path<u64>) -> ApiResult<Json<Product>> {
    catalog
        .get(id)
        .map(Json)
        .ok_or_else(|| ApiError::not_found(format!("product {id} not found")))
}

async fn create_product(
    catalog: Dep<Catalog>,
    ValidJson(new): ValidJson<NewProduct>,
) -> ApiResult<(StatusCode, Json<Product>)> {
    match catalog.create(new) {
        Ok(product) => Ok((StatusCode::CREATED, Json(product))),
        Err(err @ CatalogError::Full(_)) => Err(ApiError::conflict(err.to_string())),
    }
}

/// The application, ready to `build()`. Configuration: optional
/// `hello-api.toml`, then `HELLO__*` environment variables.
pub fn app_builder() -> ApplicationBuilder {
    Application::builder()
        .name("hello-api")
        .optional_config_file("hello-api.toml")
        .env_prefix("HELLO")
        .module(CatalogModule)
}
