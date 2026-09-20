//! Human-gate wake approvals and inbox interactions for the Ratatui TUI client (T4 / #138).
//!
//! Reuses C12 human gate approval mechanisms and audit trails for wake authorization.

use hub::{HubError, HubStore, MessageRecord, MessageStatus, PendingGateApproval};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChatViewMode {
    #[default]
    SessionMessages,
    HumanInbox,
    PendingApprovals,
}

impl ChatViewMode {
    pub fn next(self) -> Self {
        match self {
            ChatViewMode::SessionMessages => ChatViewMode::HumanInbox,
            ChatViewMode::HumanInbox => ChatViewMode::PendingApprovals,
            ChatViewMode::PendingApprovals => ChatViewMode::SessionMessages,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ChatViewMode::SessionMessages => "Messages",
            ChatViewMode::HumanInbox => "Inbox",
            ChatViewMode::PendingApprovals => "Wake Gates",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            ChatViewMode::SessionMessages => "Session Message Stream",
            ChatViewMode::HumanInbox => "Human Direct Inbox",
            ChatViewMode::PendingApprovals => "Pending Wake Gate Approvals",
        }
    }
}

#[derive(Debug, Default)]
pub struct ApprovalsState {
    pub selected_approval_index: usize,
    pub selected_inbox_index: usize,
    pub chat_view_mode: ChatViewMode,
}

impl ApprovalsState {
    pub fn select_next_approval(&mut self, total: usize) {
        if total > 0 {
            self.selected_approval_index = (self.selected_approval_index + 1) % total;
        }
    }

    pub fn select_prev_approval(&mut self, total: usize) {
        if total > 0 {
            if self.selected_approval_index == 0 {
                self.selected_approval_index = total - 1;
            } else {
                self.selected_approval_index -= 1;
            }
        }
    }

    pub fn select_next_inbox(&mut self, total: usize) {
        if total > 0 {
            self.selected_inbox_index = (self.selected_inbox_index + 1) % total;
        }
    }

    pub fn select_prev_inbox(&mut self, total: usize) {
        if total > 0 {
            if self.selected_inbox_index == 0 {
                self.selected_inbox_index = total - 1;
            } else {
                self.selected_inbox_index -= 1;
            }
        }
    }

    pub fn resolve_selected(
        &mut self,
        store: &HubStore,
        pending_gates: &[PendingGateApproval],
        approve: bool,
    ) -> Result<PendingGateApproval, HubError> {
        let gate = pending_gates
            .get(self.selected_approval_index)
            .ok_or_else(|| HubError::NotFound("No pending gate selected".into()))?;
        let resolved = store.resolve_gate_approval(&gate.id, approve)?;
        if self.selected_approval_index > 0
            && self.selected_approval_index >= pending_gates.len().saturating_sub(1)
        {
            self.selected_approval_index = self.selected_approval_index.saturating_sub(1);
        }
        Ok(resolved)
    }

    pub fn mark_selected_inbox_acked(
        &mut self,
        store: &HubStore,
        inbox_messages: &[MessageRecord],
    ) -> Result<MessageRecord, HubError> {
        let msg = inbox_messages
            .get(self.selected_inbox_index)
            .ok_or_else(|| HubError::NotFound("No inbox message selected".into()))?;
        let updated = store.set_message_status(&msg.id, MessageStatus::Acked)?;
        let _ = store.mark_read("human", &format!("channel:dm-{}", msg.from_agent), None);
        Ok(updated)
    }
}
