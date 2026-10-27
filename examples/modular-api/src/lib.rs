//! Three modules composed explicitly:
//!
//! - `accounts` owns accounts and publishes [`AccountCreated`].
//! - `audit` (depends on `accounts`) records every new account.
//! - `notifications` (depends on `accounts`) sends a welcome email through a
//!   flaky gateway as a background task with bounded retries, and drains its
//!   task runner on shutdown.
//!
//! Modules are registered in any order; Liga starts them dependencies-first.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use liga::prelude::*;
use serde::{Deserialize, Serialize};

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

// ---------------------------------------------------------------- accounts

/// Published after an account is stored. In-memory, not durable.
#[derive(Debug, Clone)]
pub struct AccountCreated {
    /// New account id.
    pub id: u64,
}

/// A stored account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// Identifier.
    pub id: u64,
    /// Normalized (trimmed, lower-case) e-mail.
    pub email: String,
}

/// Body of `POST /accounts`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewAccount {
    /// E-mail address.
    pub email: String,
}

impl Validate for NewAccount {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let email = self.email.trim();
        let well_formed = email.len() <= 254
            && email.split_once('@').is_some_and(|(local, domain)| {
                !local.is_empty() && domain.contains('.') && !domain.contains('@')
            });
        let mut errors = ValidationErrors::new();
        errors.check(
            well_formed,
            "email",
            "format",
            "must be a valid e-mail address",
        );
        errors.into_result()
    }
}

/// In-memory account store.
#[derive(Debug, Default)]
pub struct Accounts {
    accounts: Mutex<Vec<Account>>,
}

impl Accounts {
    fn create(&self, email: String) -> Account {
        let mut accounts = lock(&self.accounts);
        let account = Account {
            id: accounts.len() as u64 + 1,
            email,
        };
        accounts.push(account.clone());
        account
    }

    /// Every account.
    pub fn list(&self) -> Vec<Account> {
        lock(&self.accounts).clone()
    }
}

/// Owns accounts.
#[derive(Debug, Clone, Copy)]
pub struct AccountsModule;

impl Module for AccountsModule {
    fn name(&self) -> &'static str {
        "accounts"
    }

    fn register(&self, ctx: &mut ModuleContext<'_>) -> Result<(), ModuleError> {
        ctx.provide(Accounts::default())?;
        ctx.contribute(Routes::new().route("/accounts", get(list_accounts).post(create_account)));
        Ok(())
    }
}

async fn list_accounts(accounts: Dep<Accounts>) -> Json<Vec<Account>> {
    Json(accounts.list())
}

async fn create_account(
    accounts: Dep<Accounts>,
    events: Dep<EventBus>,
    ValidJson(new): ValidJson<NewAccount>,
) -> (StatusCode, Json<Account>) {
    let account = accounts.create(new.email.trim().to_lowercase());
    // The account is already stored: a failing subscriber is logged, not
    // turned into an error for the client.
    if let Err(err) = events.publish(AccountCreated { id: account.id }).await {
        tracing::warn!(error = %err, "some AccountCreated handlers failed");
    }
    (StatusCode::CREATED, Json(account))
}

// ------------------------------------------------------------------- audit

/// One audit record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    /// What happened.
    pub action: String,
    /// Account concerned.
    pub account_id: u64,
}

/// In-memory audit trail.
#[derive(Debug, Default)]
pub struct AuditLog {
    entries: Mutex<Vec<AuditEntry>>,
}

impl AuditLog {
    fn record(&self, entry: AuditEntry) {
        lock(&self.entries).push(entry);
    }

    /// Every entry, oldest first.
    pub fn list(&self) -> Vec<AuditEntry> {
        lock(&self.entries).clone()
    }
}

/// Records account events.
#[derive(Debug, Clone, Copy)]
pub struct AuditModule;

impl Module for AuditModule {
    fn name(&self) -> &'static str {
        "audit"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &["accounts"]
    }

    fn register(&self, ctx: &mut ModuleContext<'_>) -> Result<(), ModuleError> {
        let log = Arc::new(AuditLog::default());
        ctx.provide_arc(log.clone())?;
        ctx.events().subscribe(move |event: Arc<AccountCreated>| {
            let log = log.clone();
            async move {
                log.record(AuditEntry {
                    action: "account.created".into(),
                    account_id: event.id,
                });
                Ok(())
            }
        });
        ctx.contribute(Routes::new().route("/audit", get(list_audit)));
        Ok(())
    }
}

async fn list_audit(log: Dep<AuditLog>) -> Json<Vec<AuditEntry>> {
    Json(log.list())
}

// ----------------------------------------------------------- notifications

/// The `[notifications]` configuration section.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NotificationsConfig {
    /// E-mails sent at the same time.
    pub max_concurrent_sends: usize,
    /// Attempts per e-mail, including the first.
    pub max_attempts: u32,
    /// First retry delay; doubles each attempt.
    pub retry_base_ms: u64,
    /// Time allowed for queued e-mails on shutdown.
    pub shutdown_grace_ms: u64,
}

impl Default for NotificationsConfig {
    fn default() -> Self {
        Self {
            max_concurrent_sends: 4,
            max_attempts: 3,
            retry_base_ms: 50,
            shutdown_grace_ms: 5_000,
        }
    }
}

impl Validate for NotificationsConfig {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();
        errors
            .check(
                (1..=64).contains(&self.max_concurrent_sends),
                "max_concurrent_sends",
                "range",
                "must be 1-64",
            )
            .check(
                (1..=10).contains(&self.max_attempts),
                "max_attempts",
                "range",
                "must be 1-10",
            )
            .check(
                self.retry_base_ms <= 60_000,
                "retry_base_ms",
                "range",
                "must be at most 60000",
            );
        errors.into_result()
    }
}

/// A delivered e-mail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SentEmail {
    /// Recipient account.
    pub account_id: u64,
    /// Template name.
    pub template: String,
    /// Attempts it took.
    pub attempts: u32,
}

/// Delivered e-mails.
#[derive(Debug, Default)]
pub struct Outbox {
    sent: Mutex<Vec<SentEmail>>,
}

impl Outbox {
    fn record(&self, email: SentEmail) {
        lock(&self.sent).push(email);
    }

    /// Every delivered e-mail.
    pub fn list(&self) -> Vec<SentEmail> {
        lock(&self.sent).clone()
    }
}

/// Stand-in for an SMTP/API gateway that times out on the first attempt.
async fn send_via_flaky_gateway(attempt: u32) -> Result<(), TaskError> {
    tokio::time::sleep(Duration::from_millis(5)).await;
    if attempt == 1 {
        Err(TaskError::transient("mail gateway timed out"))
    } else {
        Ok(())
    }
}

/// Sends welcome e-mails in the background.
#[derive(Debug, Clone, Copy)]
pub struct NotificationsModule;

impl Module for NotificationsModule {
    fn name(&self) -> &'static str {
        "notifications"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &["accounts"]
    }

    fn register(&self, ctx: &mut ModuleContext<'_>) -> Result<(), ModuleError> {
        let config: NotificationsConfig = ctx.config_section("notifications")?;
        let runner = TaskRunner::builder()
            .max_concurrency(config.max_concurrent_sends)
            .default_timeout(Duration::from_secs(10))
            .build();
        let outbox = Arc::new(Outbox::default());
        let retry = RetryPolicy::exponential(
            config.max_attempts,
            Duration::from_millis(config.retry_base_ms),
        );

        ctx.provide(runner.clone())?;
        ctx.provide_arc(outbox.clone())?;
        ctx.provide(config)?;
        ctx.events().subscribe(move |event: Arc<AccountCreated>| {
            let runner = runner.clone();
            let outbox = outbox.clone();
            async move {
                let account_id = event.id;
                runner
                    .spawn(
                        TaskSpec::new("welcome-email").retry(retry),
                        move |task: TaskContext| {
                            let outbox = outbox.clone();
                            async move {
                                send_via_flaky_gateway(task.attempt()).await?;
                                outbox.record(SentEmail {
                                    account_id,
                                    template: "welcome".into(),
                                    attempts: task.attempt(),
                                });
                                Ok(())
                            }
                        },
                    )
                    .map(drop)
                    .map_err(|err| {
                        EventError::with_source("could not queue the welcome e-mail", err)
                    })
            }
        });
        ctx.contribute(Routes::new().route("/notifications", get(list_notifications)));
        Ok(())
    }

    async fn shutdown(&self, ctx: &AppContext) -> Result<(), ModuleError> {
        let runner = ctx.require::<TaskRunner>()?;
        let config = ctx.require::<NotificationsConfig>()?;
        let report = runner
            .shutdown(Duration::from_millis(config.shutdown_grace_ms))
            .await;
        if !report.drained {
            tracing::warn!(
                cancelled = report.cancelled,
                "unsent e-mails were cancelled"
            );
        }
        Ok(())
    }
}

async fn list_notifications(outbox: Dep<Outbox>) -> Json<Vec<SentEmail>> {
    Json(outbox.list())
}

/// The application. Module order here is irrelevant.
pub fn app_builder() -> ApplicationBuilder {
    Application::builder()
        .name("modular-api")
        .optional_config_file("modular-api.toml")
        .env_prefix("MODULAR")
        .module(NotificationsModule)
        .module(AuditModule)
        .module(AccountsModule)
}
