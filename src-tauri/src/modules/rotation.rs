use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::models::{Account, RotationConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationCandidate {
    pub account_id: String,
    pub email: String,
    pub quota_percentage: i32,
    pub reset_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationReason {
    pub code: String,
    pub summary: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationSuggestion {
    pub current_account_id: String,
    pub current_account_email: String,
    pub current_quota_percentage: Option<i32>,
    pub threshold_percentage: u32,
    pub target_models: Vec<String>,
    pub candidate: RotationCandidate,
    pub reason: RotationReason,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationStatus {
    pub enabled: bool,
    pub suggestion: Option<RotationSuggestion>,
    pub is_switching: bool,
    pub last_evaluated_at: Option<i64>,
}

#[derive(Debug, Default)]
struct RotationRuntimeState {
    suggestion: Option<RotationSuggestion>,
    last_evaluated_at: Option<i64>,
    suppressed_until: Option<i64>,
    last_suggestion_key: Option<String>,
}

#[derive(Debug, Default)]
pub struct RotationState {
    runtime: Mutex<RotationRuntimeState>,
    switch_in_progress: AtomicBool,
}

impl RotationState {
    pub fn status(&self, enabled: bool) -> RotationStatus {
        let runtime = self.runtime.lock();
        RotationStatus {
            enabled,
            suggestion: runtime.suggestion.clone(),
            is_switching: self.switch_in_progress.load(Ordering::SeqCst),
            last_evaluated_at: runtime.last_evaluated_at,
        }
    }

    pub fn suggestion(&self) -> Option<RotationSuggestion> {
        self.runtime.lock().suggestion.clone()
    }

    #[allow(dead_code)]
    pub fn suggestion_key(&self) -> Option<String> {
        self.runtime.lock().last_suggestion_key.clone()
    }

    pub fn set_switch_in_progress(&self, value: bool) {
        self.switch_in_progress.store(value, Ordering::SeqCst);
    }

    pub fn is_switch_in_progress(&self) -> bool {
        self.switch_in_progress.load(Ordering::SeqCst)
    }

    fn current_suggestion_key(suggestion: &RotationSuggestion) -> String {
        format!(
            "{}:{}:{}",
            suggestion.current_account_id, suggestion.candidate.account_id, suggestion.reason.code
        )
    }
}

#[derive(Debug)]
struct AccountScore {
    account: Account,
    quota_percentage: i32,
    reset_at: Option<i64>,
}

pub enum EvaluationOutcome {
    Suggested(RotationSuggestion),
    Cleared,
    NoChange,
}

pub fn evaluate_and_update_state(
    state: &RotationState,
    config: &RotationConfig,
) -> Result<EvaluationOutcome, String> {
    let now = chrono::Utc::now().timestamp();
    let suggestion = evaluate_rotation(config)?;
    let mut runtime = state.runtime.lock();
    runtime.last_evaluated_at = Some(now);

    if state.is_switch_in_progress() {
        return Ok(EvaluationOutcome::NoChange);
    }

    match suggestion {
        Some(suggestion) => {
            let suggestion_key = RotationState::current_suggestion_key(&suggestion);
            if let Some(until) = runtime.suppressed_until {
                if until > now && runtime.last_suggestion_key.as_deref() == Some(&suggestion_key) {
                    return Ok(EvaluationOutcome::NoChange);
                }
            }

            let changed = runtime.last_suggestion_key.as_deref() != Some(&suggestion_key);
            runtime.last_suggestion_key = Some(suggestion_key);
            runtime.suggestion = Some(suggestion.clone());
            if changed {
                Ok(EvaluationOutcome::Suggested(suggestion))
            } else {
                Ok(EvaluationOutcome::NoChange)
            }
        }
        None => {
            let had_suggestion = runtime.suggestion.take().is_some();
            runtime.last_suggestion_key = None;
            runtime.suppressed_until = None;
            if had_suggestion {
                Ok(EvaluationOutcome::Cleared)
            } else {
                Ok(EvaluationOutcome::NoChange)
            }
        }
    }
}

pub fn dismiss_suggestion(
    state: &RotationState,
    config: &RotationConfig,
    remind_after_seconds: Option<u64>,
) -> bool {
    let mut runtime = state.runtime.lock();
    if runtime.suggestion.is_none() {
        return false;
    }
    let now = chrono::Utc::now().timestamp();
    let remind_after = remind_after_seconds.unwrap_or(config.cooldown_seconds) as i64;
    runtime.suppressed_until = Some(now + remind_after.max(0));
    runtime.suggestion = None;
    true
}

pub fn clear_suggestion(state: &RotationState) -> bool {
    let mut runtime = state.runtime.lock();
    let changed = runtime.suggestion.take().is_some() || runtime.last_suggestion_key.take().is_some();
    runtime.suppressed_until = None;
    changed
}

fn evaluate_rotation(config: &RotationConfig) -> Result<Option<RotationSuggestion>, String> {
    if !config.enabled || config.target_models.is_empty() {
        return Ok(None);
    }

    let Some(current_account_id) = crate::modules::get_current_account_id()? else {
        return Ok(None);
    };

    let current_account = crate::modules::load_account(&current_account_id)?;
    let target_models = normalize_models(&config.target_models);

    let current_issue = detect_current_issue(&current_account, &target_models, config);
    let Some((reason, current_quota)) = current_issue else {
        return Ok(None);
    };

    let accounts = crate::modules::list_accounts()?;
    let candidate = select_candidate(
        accounts,
        &current_account_id,
        &target_models,
        config.quota_threshold_percentage as i32,
    );

    let Some(candidate) = candidate else {
        return Ok(None);
    };

    Ok(Some(RotationSuggestion {
        current_account_id: current_account.id.clone(),
        current_account_email: current_account.email.clone(),
        current_quota_percentage: current_quota,
        threshold_percentage: config.quota_threshold_percentage,
        target_models: config.target_models.clone(),
        candidate: RotationCandidate {
            account_id: candidate.account.id.clone(),
            email: candidate.account.email.clone(),
            quota_percentage: candidate.quota_percentage,
            reset_at: candidate.reset_at,
        },
        reason,
        created_at: chrono::Utc::now().timestamp(),
    }))
}

fn detect_current_issue(
    account: &Account,
    target_models: &HashSet<String>,
    config: &RotationConfig,
) -> Option<(RotationReason, Option<i32>)> {
    if config.trigger_on_forbidden && account.quota.as_ref().map(|q| q.is_forbidden).unwrap_or(false) {
        return Some((
            RotationReason {
                code: "forbidden".to_string(),
                summary: "Current account is forbidden".to_string(),
                detail: format!("{} can no longer serve the target model.", account.email),
            },
            None,
        ));
    }

    if config.trigger_on_validation_blocked && account.validation_blocked {
        return Some((
            RotationReason {
                code: "validation_blocked".to_string(),
                summary: "Current account requires verification".to_string(),
                detail: format!("{} is temporarily blocked pending validation.", account.email),
            },
            None,
        ));
    }

    if account.disabled || account.proxy_disabled || is_protected_for_targets(account, target_models) {
        return Some((
            RotationReason {
                code: "target_unavailable".to_string(),
                summary: "Current account is not eligible for the target model".to_string(),
                detail: format!("{} is disabled or reserved for protected quota.", account.email),
            },
            best_target_quota(account, target_models).map(|(quota, _)| quota),
        ));
    }

    let (quota, _) = best_target_quota(account, target_models).unwrap_or((-1, None));
    if quota < config.quota_threshold_percentage as i32 {
        return Some((
            RotationReason {
                code: "quota_low".to_string(),
                summary: "Current account quota is below threshold".to_string(),
                detail: format!(
                    "{} target quota is {}%, below {}%.",
                    account.email, quota.max(0), config.quota_threshold_percentage
                ),
            },
            if quota >= 0 { Some(quota) } else { None },
        ));
    }

    None
}

fn select_candidate(
    accounts: Vec<Account>,
    current_account_id: &str,
    target_models: &HashSet<String>,
    threshold: i32,
) -> Option<AccountScore> {
    let mut candidates: Vec<AccountScore> = accounts
        .into_iter()
        .filter(|account| account.id != current_account_id)
        .filter(|account| !account.disabled)
        .filter(|account| !account.proxy_disabled)
        .filter(|account| !account.validation_blocked)
        .filter(|account| !account.quota.as_ref().map(|q| q.is_forbidden).unwrap_or(false))
        .filter(|account| !is_protected_for_targets(account, target_models))
        .filter_map(|account| {
            let (quota, reset_at) = best_target_quota(&account, target_models)?;
            if quota < threshold {
                return None;
            }
            Some(AccountScore {
                account,
                quota_percentage: quota,
                reset_at,
            })
        })
        .collect();

    candidates.sort_by(|a, b| {
        b.quota_percentage
            .cmp(&a.quota_percentage)
            .then_with(|| a.reset_at.unwrap_or(i64::MAX).cmp(&b.reset_at.unwrap_or(i64::MAX)))
            .then_with(|| a.account.last_used.cmp(&b.account.last_used))
    });

    candidates.into_iter().next()
}

fn normalize_models(models: &[String]) -> HashSet<String> {
    models
        .iter()
        .map(|model| model.trim().to_lowercase())
        .filter(|model| !model.is_empty())
        .collect()
}

fn is_protected_for_targets(account: &Account, target_models: &HashSet<String>) -> bool {
    account
        .protected_models
        .iter()
        .map(|model| model.trim().to_lowercase())
        .any(|model| target_models.contains(&model))
}

fn best_target_quota(account: &Account, target_models: &HashSet<String>) -> Option<(i32, Option<i64>)> {
    let quota = account.quota.as_ref()?;
    quota.models
        .iter()
        .filter(|model| target_models.contains(&model.name.trim().to_lowercase()))
        .map(|model| (model.percentage, parse_reset_time(&model.reset_time)))
        .max_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.unwrap_or(i64::MAX).cmp(&a.1.unwrap_or(i64::MAX))))
}

fn parse_reset_time(input: &str) -> Option<i64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(timestamp) = trimmed.parse::<i64>() {
        return Some(timestamp);
    }
    chrono::DateTime::parse_from_rfc3339(trimmed)
        .map(|dt| dt.timestamp())
        .ok()
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|dt| dt.and_utc().timestamp())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{QuotaData, TokenData};
    use crate::models::quota::ModelQuota;

    fn make_account(id: &str, email: &str, quota: i32) -> Account {
        let mut account = Account::new(
            id.to_string(),
            email.to_string(),
            TokenData::new(
                "access".to_string(),
                "refresh".to_string(),
                3600,
                Some(email.to_string()),
                None,
                None,
                true,
            ),
        );
        account.quota = Some(QuotaData {
            models: vec![ModelQuota {
                name: "gemini-3-pro-high".to_string(),
                percentage: quota,
                reset_time: "2099-01-01T00:00:00Z".to_string(),
                display_name: None,
                supports_images: None,
                supports_thinking: None,
                thinking_budget: None,
                recommended: None,
                max_tokens: None,
                max_output_tokens: None,
                supported_mime_types: None,
            }],
            last_updated: 0,
            is_forbidden: false,
            forbidden_reason: None,
            subscription_tier: Some("ULTRA".to_string()),
            model_forwarding_rules: Default::default(),
        });
        account
    }

    #[test]
    fn best_candidate_prefers_higher_quota() {
        let current = "a1".to_string();
        let targets = normalize_models(&["gemini-3-pro-high".to_string()]);
        let accounts = vec![
            make_account("a1", "a1@test", 10),
            make_account("a2", "a2@test", 50),
            make_account("a3", "a3@test", 80),
        ];
        let candidate = select_candidate(accounts, &current, &targets, 15).expect("candidate");
        assert_eq!(candidate.account.id, "a3");
    }

    #[test]
    fn protected_account_is_rejected() {
        let mut account = make_account("a1", "a1@test", 10);
        account.protected_models = std::iter::once("gemini-3-pro-high".to_string()).collect();
        let targets = normalize_models(&["gemini-3-pro-high".to_string()]);
        assert!(is_protected_for_targets(&account, &targets));
    }
}
