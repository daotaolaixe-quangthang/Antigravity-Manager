use serde::{Deserialize, Serialize};
use crate::proxy::ProxyConfig;
use crate::modules::cloudflared::CloudflaredConfig;

fn default_true() -> bool {
    true
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub language: String,
    pub theme: String,
    pub auto_refresh: bool,
    pub refresh_interval: i32,  // minutes
    pub auto_sync: bool,
    pub sync_interval: i32,  // minutes
    pub default_export_path: Option<String>,
    #[serde(default)]
    pub proxy: ProxyConfig,
    pub antigravity_executable: Option<String>, // [NEW] Manually specified Antigravity executable path
    pub antigravity_args: Option<Vec<String>>, // [NEW] Antigravity startup arguments
    #[serde(default)]
    pub auto_launch: bool,  // Launch on startup
    #[serde(default)]
    pub scheduled_warmup: ScheduledWarmupConfig, // [NEW] Scheduled warmup configuration
    #[serde(default)]
    pub quota_protection: QuotaProtectionConfig, // [NEW] Quota protection configuration
    #[serde(default)]
    pub pinned_quota_models: PinnedQuotaModelsConfig, // [NEW] Pinned quota models list
    #[serde(default)]
    pub circuit_breaker: CircuitBreakerConfig, // [NEW] Circuit breaker configuration
    #[serde(default)]
    pub rotation: RotationConfig, // [NEW] Native IDE semi-auto rotation
    #[serde(default)]
    pub hidden_menu_items: Vec<String>, // Hidden menu item path list
    #[serde(default)]
    pub cloudflared: CloudflaredConfig, // [NEW] Cloudflared configuration
}

/// Scheduled warmup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledWarmupConfig {
    /// Whether smart warmup is enabled
    pub enabled: bool,

    /// List of models to warmup
    #[serde(default = "default_warmup_models")]
    pub monitored_models: Vec<String>,
}

fn default_warmup_models() -> Vec<String> {
    vec![
        "gemini-3-flash".to_string(),
        "claude".to_string(),
        "gemini-3-pro-high".to_string(),
        "gemini-3-pro-image".to_string(),
    ]
}

impl ScheduledWarmupConfig {
    pub fn new() -> Self {
        Self {
            enabled: false,
            monitored_models: default_warmup_models(),
        }
    }
}

impl Default for ScheduledWarmupConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Quota protection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaProtectionConfig {
    /// Whether quota protection is enabled
    pub enabled: bool,
    
    /// Reserved quota percentage (1-99)
    pub threshold_percentage: u32,

    /// List of monitored models (e.g. gemini-3-flash, gemini-3-pro-high, gemini-3.1-pro-high, claude-sonnet-4-6)
    #[serde(default = "default_monitored_models")]
    pub monitored_models: Vec<String>,
}

fn default_monitored_models() -> Vec<String> {
    vec![
        "claude".to_string(),
        "gemini-3-pro-high".to_string(),
        "gemini-3-flash".to_string(),
        "gemini-3-pro-image".to_string(),
    ]
}

impl QuotaProtectionConfig {
    pub fn new() -> Self {
        Self {
            enabled: false,
            threshold_percentage: 10, // Default 10% reserve
            monitored_models: default_monitored_models(),
        }
    }
}

impl Default for QuotaProtectionConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Pinned quota models configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedQuotaModelsConfig {
    /// List of pinned models (displayed outside the account list)
    #[serde(default = "default_pinned_models")]
    pub models: Vec<String>,
}

fn default_pinned_models() -> Vec<String> {
    vec![
        "gemini-3-pro-high".to_string(),
        "gemini-3-flash".to_string(),
        "gemini-3-pro-image".to_string(),
        "claude-sonnet-4-6-thinking".to_string(),
    ]
}

impl PinnedQuotaModelsConfig {
    pub fn new() -> Self {
        Self {
            models: default_pinned_models(),
        }
    }
}

impl Default for PinnedQuotaModelsConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Whether circuit breaker is enabled
    pub enabled: bool,

    /// Unified backoff steps (seconds)
    /// Default: [60, 300, 1800, 7200]
    #[serde(default = "default_backoff_steps")]
    pub backoff_steps: Vec<u64>,
}

fn default_backoff_steps() -> Vec<u64> {
    vec![60, 300, 1800, 7200]
}

impl CircuitBreakerConfig {
    pub fn new() -> Self {
        Self {
            enabled: true,
            backoff_steps: default_backoff_steps(),
        }
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Native IDE semi-auto rotation notification channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationNotificationChannels {
    #[serde(default = "default_true")]
    pub tray: bool,
    #[serde(default = "default_true")]
    pub popup: bool,
}

impl Default for RotationNotificationChannels {
    fn default() -> Self {
        Self {
            tray: true,
            popup: true,
        }
    }
}

/// Native IDE semi-auto rotation config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_rotation_mode")]
    pub mode: String,
    #[serde(default = "default_rotation_target_models")]
    pub target_models: Vec<String>,
    #[serde(default = "default_rotation_threshold")]
    pub quota_threshold_percentage: u32,
    #[serde(default = "default_true")]
    pub trigger_on_forbidden: bool,
    #[serde(default = "default_true")]
    pub trigger_on_validation_blocked: bool,
    #[serde(default = "default_rotation_cooldown")]
    pub cooldown_seconds: u64,
    #[serde(default)]
    pub notification_channels: RotationNotificationChannels,
    #[serde(default = "default_true")]
    pub require_confirmation: bool,
}

fn default_rotation_mode() -> String {
    "semi_auto".to_string()
}

fn default_rotation_target_models() -> Vec<String> {
    vec![
        "gemini-3.1-pro-high".to_string(),
        "gemini-3-pro-high".to_string(),
    ]
}

fn default_rotation_threshold() -> u32 {
    15
}

fn default_rotation_cooldown() -> u64 {
    600
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: default_rotation_mode(),
            target_models: default_rotation_target_models(),
            quota_threshold_percentage: default_rotation_threshold(),
            trigger_on_forbidden: true,
            trigger_on_validation_blocked: true,
            cooldown_seconds: default_rotation_cooldown(),
            notification_channels: RotationNotificationChannels::default(),
            require_confirmation: true,
        }
    }
}

impl AppConfig {
    pub fn new() -> Self {
        Self {
            language: "en".to_string(),
            theme: "system".to_string(),
            auto_refresh: true,
            refresh_interval: 15,
            auto_sync: false,
            sync_interval: 5,
            default_export_path: None,
            proxy: ProxyConfig::default(),
            antigravity_executable: None,
            antigravity_args: None,
            auto_launch: false,
            scheduled_warmup: ScheduledWarmupConfig::default(),
            quota_protection: QuotaProtectionConfig::default(),
            pinned_quota_models: PinnedQuotaModelsConfig::default(),
            circuit_breaker: CircuitBreakerConfig::default(),
            rotation: RotationConfig::default(),
            hidden_menu_items: Vec::new(),
            cloudflared: CloudflaredConfig::default(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::new()
    }
}
