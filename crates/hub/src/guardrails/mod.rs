//! Guardrails primitive (P16 / #360, `roadmaps/platform.md`).
//!
//! Content screening for what the hub dispatches and what it accepts back,
//! adapted from `openai-agents-python`'s input/output guardrails to this
//! app's dispatch model:
//!
//! * **Input** guardrails run *before* a task step or scheduled prompt is
//!   sent. A tripwire there means no message row, no wake, no schedule
//!   attempt claimed — nothing was spent on something that could never be
//!   acceptable.
//! * **Output** guardrails run on an agent's reply *before* the hub acts on
//!   it: before a workflow stage advances on it, and before a Schedule
//!   gate is evaluated against it.
//!
//! Harness-CLI agents write their replies into `messages` themselves, so
//! an output guardrail cannot un-write a row; what it halts is
//! *acceptance* — the stage does not advance, the gate is not evaluated.
//!
//! This is **not** a new approval-gate class. It sits before P5's sandbox
//! approval and before Schedule's `GateCriterion`; it never replaces or
//! bypasses them. A tripwire never asks a human — it halts, writes one
//! `guardrail.tripped` audit row (rule, stage, subject, detail; never the
//! screened content), and the caller sees an error.
//!
//! Every implementor is one [`Guardrail`]. The built-ins here are plain
//! predicates ([`GuardrailSpec`]); a P5 command check or an LLM-backed
//! check is just another implementor of the same trait, not a different
//! primitive. Rules are stored per hub (global) or per workspace
//! (`HubStore::add_guardrail`), screened by `HubStore::screen_content`.

mod builtin;

use serde::{Deserialize, Serialize};

pub use builtin::GuardrailSpec;

/// Where a rule applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardrailStage {
    Input,
    Output,
}

impl GuardrailStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Output => "output",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "input" => Some(Self::Input),
            "output" => Some(Self::Output),
            _ => None,
        }
    }
}

/// How a rule is scheduled relative to its siblings. Both modes halt
/// before the content is used. `Blocking` rules run first, in creation
/// order, and the first tripwire stops everything else. `Parallel` rules
/// run concurrently with each other once every blocking rule passed, and
/// all of their trips are reported — the mode for slow checks that are
/// worth overlapping, not a way to let content through early.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardrailMode {
    #[default]
    Blocking,
    Parallel,
}

impl GuardrailMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blocking => "blocking",
            Self::Parallel => "parallel",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "blocking" => Some(Self::Blocking),
            "parallel" => Some(Self::Parallel),
            _ => None,
        }
    }
}

/// Outcome of one check. `tripwire_triggered` is the halt signal;
/// `passed = false` without a tripwire is an advisory observation that
/// does not stop anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardrailResult {
    pub passed: bool,
    pub tripwire_triggered: bool,
    pub detail: String,
}

impl GuardrailResult {
    pub fn pass(detail: impl Into<String>) -> Self {
        Self {
            passed: true,
            tripwire_triggered: false,
            detail: bounded(detail.into()),
        }
    }

    pub fn trip(detail: impl Into<String>) -> Self {
        Self {
            passed: false,
            tripwire_triggered: true,
            detail: bounded(detail.into()),
        }
    }
}

/// One screening check. Implementors must never copy the screened content
/// into `detail` — details end up in the audit chain.
pub trait Guardrail: Send + Sync {
    fn name(&self) -> &str;
    fn check(&self, content: &str) -> GuardrailResult;
}

/// A stored rule: a named, enabled-or-not [`GuardrailSpec`] bound to a
/// stage, a mode, and a hub-wide (`workspace = None`) or per-workspace
/// scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuardrailRule {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub stage: GuardrailStage,
    #[serde(default)]
    pub mode: GuardrailMode,
    pub spec: GuardrailSpec,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardrailTrip {
    pub rule_id: String,
    pub name: String,
    pub mode: GuardrailMode,
    pub detail: String,
}

/// Result of screening one piece of content against a rule set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardrailVerdict {
    /// Rules actually evaluated (a blocking tripwire stops the rest).
    pub checked: usize,
    pub trips: Vec<GuardrailTrip>,
}

impl GuardrailVerdict {
    pub fn tripped(&self) -> Option<&GuardrailTrip> {
        self.trips.first()
    }

    pub fn halted(&self) -> bool {
        !self.trips.is_empty()
    }
}

/// Screen `content` against `rules` (already filtered to one stage and
/// scope). Pure: no store, no audit — `HubStore::screen_content` adds
/// those.
pub fn run_rules(rules: &[GuardrailRule], content: &str) -> GuardrailVerdict {
    let mut verdict = GuardrailVerdict::default();
    let live: Vec<&GuardrailRule> = rules.iter().filter(|rule| rule.enabled).collect();

    for rule in live
        .iter()
        .filter(|rule| rule.mode == GuardrailMode::Blocking)
    {
        verdict.checked += 1;
        let result = rule.spec.build(&rule.name).check(content);
        if result.tripwire_triggered {
            verdict.trips.push(GuardrailTrip {
                rule_id: rule.id.clone(),
                name: rule.name.clone(),
                mode: rule.mode,
                detail: result.detail,
            });
            return verdict;
        }
    }

    let parallel: Vec<&GuardrailRule> = live
        .iter()
        .copied()
        .filter(|rule| rule.mode == GuardrailMode::Parallel)
        .collect();
    if parallel.is_empty() {
        return verdict;
    }
    verdict.checked += parallel.len();
    let results: Vec<GuardrailResult> = std::thread::scope(|scope| {
        let handles: Vec<_> = parallel
            .iter()
            .map(|rule| scope.spawn(move || rule.spec.build(&rule.name).check(content)))
            .collect();
        handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .unwrap_or_else(|_| GuardrailResult::trip("guardrail panicked; failing closed"))
            })
            .collect()
    });
    for (rule, result) in parallel.iter().zip(results) {
        if result.tripwire_triggered {
            verdict.trips.push(GuardrailTrip {
                rule_id: rule.id.clone(),
                name: rule.name.clone(),
                mode: rule.mode,
                detail: result.detail,
            });
        }
    }
    verdict
}

const DETAIL_LIMIT: usize = 240;

pub(crate) fn bounded(mut text: String) -> String {
    if text.chars().count() <= DETAIL_LIMIT {
        return text;
    }
    text = text.chars().take(DETAIL_LIMIT).collect();
    text.push('…');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(name: &str, mode: GuardrailMode, spec: GuardrailSpec) -> GuardrailRule {
        GuardrailRule {
            id: format!("id-{name}"),
            name: name.into(),
            workspace: None,
            stage: GuardrailStage::Input,
            mode,
            spec,
            enabled: true,
            created_at: "2026-10-20T00:00:00Z".into(),
        }
    }

    #[test]
    fn blocking_tripwire_short_circuits_everything_after_it() {
        let rules = vec![
            rule(
                "deny",
                GuardrailMode::Blocking,
                GuardrailSpec::DenyPhrases {
                    phrases: vec!["rm -rf".into()],
                    case_insensitive: true,
                },
            ),
            rule(
                "len",
                GuardrailMode::Blocking,
                GuardrailSpec::MaxChars { max: 5 },
            ),
            rule(
                "par",
                GuardrailMode::Parallel,
                GuardrailSpec::MaxChars { max: 5 },
            ),
        ];
        let verdict = run_rules(&rules, "please RM -RF the build dir");
        assert_eq!(verdict.checked, 1);
        assert_eq!(verdict.trips.len(), 1);
        assert_eq!(verdict.tripped().unwrap().name, "deny");
    }

    #[test]
    fn parallel_rules_all_run_and_all_report_after_blocking_passes() {
        let rules = vec![
            rule(
                "len",
                GuardrailMode::Blocking,
                GuardrailSpec::MaxChars { max: 100 },
            ),
            rule(
                "p1",
                GuardrailMode::Parallel,
                GuardrailSpec::MaxChars { max: 3 },
            ),
            rule(
                "p2",
                GuardrailMode::Parallel,
                GuardrailSpec::RequirePhrases {
                    phrases: vec!["DONE".into()],
                    case_insensitive: false,
                },
            ),
            rule(
                "p3",
                GuardrailMode::Parallel,
                GuardrailSpec::MaxChars { max: 100 },
            ),
        ];
        let verdict = run_rules(&rules, "short reply");
        assert_eq!(verdict.checked, 4);
        let names: Vec<&str> = verdict.trips.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["p1", "p2"]);
        assert!(verdict.halted());
    }

    #[test]
    fn disabled_rules_are_not_evaluated() {
        let mut off = rule(
            "off",
            GuardrailMode::Blocking,
            GuardrailSpec::MaxChars { max: 1 },
        );
        off.enabled = false;
        let verdict = run_rules(&[off], "anything");
        assert_eq!(verdict.checked, 0);
        assert!(!verdict.halted());
    }

    #[test]
    fn stage_and_mode_round_trip_their_strings() {
        for stage in [GuardrailStage::Input, GuardrailStage::Output] {
            assert_eq!(GuardrailStage::parse(stage.as_str()), Some(stage));
        }
        for mode in [GuardrailMode::Blocking, GuardrailMode::Parallel] {
            assert_eq!(GuardrailMode::parse(mode.as_str()), Some(mode));
        }
        assert_eq!(GuardrailStage::parse("sideways"), None);
    }
}
