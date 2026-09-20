//! Message composer for the Ratatui TUI client (T4 / #138).
//!
//! Provides desktop-parity All/Subset/One recipient selection, [TASK] and [WAKE]
//! intent tagging, confirmation defaults, C11 validation, and delivery outcome tracking.

use hub::{
    AgentRecord, HubError, HubStore, MessageKind, MessageRecord, SendOutcome, WorkSessionRecord,
};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipientMode {
    All,
    Subset,
    Single,
}

impl RecipientMode {
    pub fn next(self) -> Self {
        match self {
            RecipientMode::All => RecipientMode::Subset,
            RecipientMode::Subset => RecipientMode::Single,
            RecipientMode::Single => RecipientMode::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            RecipientMode::All => "All (Broadcast)",
            RecipientMode::Subset => "Subset",
            RecipientMode::Single => "Single",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmationPrompt {
    Wake { targets: Vec<String> },
    Broadcast { targets: Vec<String> },
    AutoEnroll { targets: Vec<String> },
}

impl ConfirmationPrompt {
    pub fn description(&self) -> String {
        match self {
            ConfirmationPrompt::Wake { targets } => {
                format!("Confirm wake request for: {}? (y/n)", targets.join(", "))
            }
            ConfirmationPrompt::Broadcast { targets } => {
                format!("Confirm broadcast send to {} agents? (y/n)", targets.len())
            }
            ConfirmationPrompt::AutoEnroll { targets } => {
                format!(
                    "Confirm auto-enrollment and wake for: {}? (y/n)",
                    targets.join(", ")
                )
            }
        }
    }
}

pub struct ComposerState {
    pub is_open: bool,
    pub body: String,
    pub recipient_mode: RecipientMode,
    pub selected_subset: BTreeMap<String, bool>,
    pub single_recipient: String,
    pub subset_cursor: usize,
    pub is_task_tag: bool,
    pub is_wake_tag: bool,
    pub confirmation_prompt: Option<ConfirmationPrompt>,
    pub delivery_outcomes: Option<Vec<SendOutcome>>,
    pub outcome_scroll: usize,
    pub error_message: Option<String>,
}

impl Default for ComposerState {
    fn default() -> Self {
        Self {
            is_open: false,
            body: String::new(),
            recipient_mode: RecipientMode::All,
            selected_subset: BTreeMap::new(),
            single_recipient: String::from("grok"),
            subset_cursor: 0,
            is_task_tag: false,
            is_wake_tag: false,
            confirmation_prompt: None,
            delivery_outcomes: None,
            outcome_scroll: 0,
            error_message: None,
        }
    }
}

impl ComposerState {
    pub fn open(&mut self, available_agents: &[AgentRecord]) {
        self.is_open = true;
        self.error_message = None;
        self.confirmation_prompt = None;
        self.delivery_outcomes = None;
        if self.selected_subset.is_empty() {
            for agent in available_agents {
                if agent.id != "human" && agent.id != "system" {
                    self.selected_subset.insert(agent.id.clone(), true);
                }
            }
        }
        if self.single_recipient.is_empty() {
            if let Some(first) = available_agents
                .iter()
                .find(|a| a.id != "human" && a.id != "system")
            {
                self.single_recipient = first.id.clone();
            }
        }
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.confirmation_prompt = None;
        self.error_message = None;
    }

    pub fn toggle_task_tag(&mut self) {
        self.is_task_tag = !self.is_task_tag;
    }

    pub fn toggle_wake_tag(&mut self) {
        self.is_wake_tag = !self.is_wake_tag;
    }

    pub fn cycle_recipient_mode(&mut self) {
        self.recipient_mode = self.recipient_mode.next();
    }

    /// Resolves target recipients based on recipient mode and available members.
    pub fn resolve_targets(
        &self,
        enrolled_roster: &[AgentRecord],
        session: Option<&WorkSessionRecord>,
    ) -> Vec<String> {
        let eligible: Vec<String> = if let Some(sess) = session {
            sess.member_ids
                .iter()
                .filter(|id| id.as_str() != "human" && id.as_str() != "system")
                .cloned()
                .collect()
        } else {
            enrolled_roster
                .iter()
                .filter(|a| a.id != "human" && a.id != "system")
                .map(|a| a.id.clone())
                .collect()
        };

        match self.recipient_mode {
            RecipientMode::All => eligible,
            RecipientMode::Subset => eligible
                .into_iter()
                .filter(|id| *self.selected_subset.get(id).unwrap_or(&true))
                .collect(),
            RecipientMode::Single => {
                if self.single_recipient.is_empty() {
                    vec![]
                } else {
                    vec![self.single_recipient.clone()]
                }
            }
        }
    }

    /// Validates C10-C13 rules before sending.
    /// Returns Ok(true) if ready to send immediately, Ok(false) if confirmation required,
    /// or Err(message) on validation failure.
    pub fn check_confirmation_and_validation(
        &mut self,
        enrolled_roster: &[AgentRecord],
        session: Option<&WorkSessionRecord>,
    ) -> Result<bool, String> {
        if self.body.trim().is_empty() {
            return Err(String::from("Message body must not be empty."));
        }

        let targets = self.resolve_targets(enrolled_roster, session);
        if targets.is_empty() {
            return Err(String::from(
                "Please select at least one recipient agent before sending.",
            ));
        }

        let enrolled_ids: Vec<&str> = enrolled_roster.iter().map(|a| a.id.as_str()).collect();

        // C11 Validation: Task-tagged messages MUST target currently present members.
        if self.is_task_tag {
            let non_present: Vec<String> = targets
                .iter()
                .filter(|t| !enrolled_ids.contains(&t.as_str()))
                .cloned()
                .collect();
            if !non_present.is_empty() {
                return Err(format!(
                    "Task target not on team: {}. Task stays non-spawning; use [WAKE] to enroll new agents.",
                    non_present.join(", ")
                ));
            }
        }

        // Confirmation defaults: Wakes, broadcasts, and new auto-enrollments require confirmation.
        if self.is_wake_tag {
            self.confirmation_prompt = Some(ConfirmationPrompt::Wake { targets });
            return Ok(false);
        }

        let auto_enroll_targets: Vec<String> = targets
            .iter()
            .filter(|t| !enrolled_ids.contains(&t.as_str()))
            .cloned()
            .collect();
        if !auto_enroll_targets.is_empty() {
            self.confirmation_prompt = Some(ConfirmationPrompt::AutoEnroll {
                targets: auto_enroll_targets,
            });
            return Ok(false);
        }

        if targets.len() > 1 || self.recipient_mode == RecipientMode::All {
            self.confirmation_prompt = Some(ConfirmationPrompt::Broadcast { targets });
            return Ok(false);
        }

        Ok(true)
    }

    /// Executes the actual Send action through HubStore.
    pub fn execute_send(
        &mut self,
        store: &HubStore,
        workspace: Option<&Path>,
        active_session_id: Option<&str>,
        enrolled_roster: &[AgentRecord],
        session: Option<&WorkSessionRecord>,
    ) -> Result<Vec<SendOutcome>, HubError> {
        let targets = self.resolve_targets(enrolled_roster, session);
        let mut body_text = self.body.trim().to_string();

        if self.is_task_tag && !body_text.starts_with("[TASK]") {
            body_text = format!("[TASK] {body_text}");
        }
        if self.is_wake_tag && !body_text.starts_with("[WAKE]") {
            body_text = format!("[WAKE] {body_text}");
        }

        let ws_str = workspace.map(|p| p.display().to_string());
        let from_agent = "human";

        let outcomes = if self.is_task_tag || self.is_wake_tag {
            store.send_tagged_message(
                from_agent,
                &targets,
                self.is_task_tag,
                self.is_wake_tag,
                &body_text,
                None,
                ws_str.as_deref(),
                None,
                active_session_id,
            )?
        } else if let Some(sess_id) = active_session_id {
            let messages = store.send_session_message(
                from_agent,
                sess_id,
                &targets,
                &body_text,
                None,
                ws_str.as_deref(),
                None,
            )?;
            messages_to_outcomes(&messages, &targets, from_agent)
        } else {
            let mut outcomes = Vec::new();
            for target in &targets {
                let msg = store.send_message(
                    from_agent,
                    target,
                    MessageKind::Message,
                    &body_text,
                    None,
                    ws_str.as_deref(),
                    None,
                )?;
                outcomes.push(single_message_outcome(&msg, from_agent, target));
            }
            outcomes
        };

        // Reset composer inputs
        self.body.clear();
        self.is_task_tag = false;
        self.is_wake_tag = false;
        self.confirmation_prompt = None;
        self.error_message = None;
        self.delivery_outcomes = Some(outcomes.clone());

        Ok(outcomes)
    }
}

fn messages_to_outcomes(
    messages: &[MessageRecord],
    targets: &[String],
    from_agent: &str,
) -> Vec<SendOutcome> {
    messages
        .iter()
        .zip(targets.iter())
        .map(|(msg, target)| SendOutcome {
            id: msg.id.clone(),
            subject: msg.subject.clone().unwrap_or_default(),
            from_agent: from_agent.to_string(),
            to_agent: target.clone(),
            is_task: false,
            is_wake: false,
            accepted: true,
            enrolled: false,
            wake_requested: false,
            reason: None,
            policy_decision: String::from("delivered"),
            message_id: Some(msg.id.clone()),
            created_at: msg.created_at.clone(),
        })
        .collect()
}

fn single_message_outcome(
    msg: &MessageRecord,
    from_agent: &str,
    target_agent: &str,
) -> SendOutcome {
    SendOutcome {
        id: msg.id.clone(),
        subject: msg.subject.clone().unwrap_or_default(),
        from_agent: from_agent.to_string(),
        to_agent: target_agent.to_string(),
        is_task: false,
        is_wake: false,
        accepted: true,
        enrolled: false,
        wake_requested: false,
        reason: None,
        policy_decision: String::from("delivered"),
        message_id: Some(msg.id.clone()),
        created_at: msg.created_at.clone(),
    }
}
