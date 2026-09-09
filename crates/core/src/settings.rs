//! Typed application settings (filled in Phase 2; key constants live here now).

/// Well-known setting keys stored in `app_settings`.
pub mod keys {
    pub const WOW_DIR: &str = "wow_dir";
    pub const REGION: &str = "region";
    pub const MARKET_PROVIDER: &str = "market_provider";
    pub const MARKET_REFRESH_HOURS: &str = "market_refresh_hours";
    pub const CROWD_SHARE: &str = "crowd.share";
    pub const CROWD_USE: &str = "crowd.use";
    pub const CROWD_ENDPOINT: &str = "crowd.endpoint";
    pub const ROUTE_CONFIG: &str = "route.config";
    pub const MOVEMENT_PROFILE: &str = "route.movement_profile";
    pub const UI_LAST_ROUTE: &str = "ui.last_route";
}
