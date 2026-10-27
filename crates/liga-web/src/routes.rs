use axum::Router;
use axum::routing::MethodRouter;
use liga_core::AppContext;

/// Routes contributed by a module. Handlers get the [`AppContext`] as Axum state.
///
/// ```
/// use liga_web::{Routes, axum::routing::get};
///
/// let routes = Routes::new().route("/ping", get(|| async { "pong" }));
/// # let _ = routes;
/// ```
///
/// Like Axum, `route` panics on invalid or duplicated paths within one
/// `Routes`. Conflicts *between* modules are reported by
/// [`build_router`](crate::build_router) as an error.
#[derive(Debug, Default)]
pub struct Routes {
    router: Router<AppContext>,
}

impl Routes {
    /// Empty route set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a route.
    pub fn route(mut self, path: &str, method_router: MethodRouter<AppContext>) -> Self {
        self.router = self.router.route(path, method_router);
        self
    }

    /// Mounts `routes` under a path prefix.
    pub fn nest(mut self, prefix: &str, routes: Routes) -> Self {
        self.router = self.router.nest(prefix, routes.router);
        self
    }

    /// Merges another route set into this one.
    pub fn merge(mut self, other: Routes) -> Self {
        self.router = self.router.merge(other.router);
        self
    }

    /// Wraps an existing Axum router.
    pub fn from_router(router: Router<AppContext>) -> Self {
        Self { router }
    }

    /// Unwraps into the Axum router.
    pub fn into_router(self) -> Router<AppContext> {
        self.router
    }
}
