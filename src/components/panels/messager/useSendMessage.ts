import type { MutableRefObject } from "react";
import { invoke } from "../../../lib/tauri";
import type { HarnessDeliveryNotice } from "../harness/types";
import { deliverTaggedSession } from "./sendTagged";
import { rosterAgentIds, teamWakeTargets } from "./utils";
import type { HubAgent, PendingAttachment, ReplyTarget, WorkSession } from "./types";

export function useSendMessage(params: {
  activeChannel: string;
  activeWorkSession: WorkSession | null;
  hubAgents: HubAgent[];
  workspacePath: string;
  messageInput: string;
  sending: boolean;
  setSending: (value: boolean) => void;
  recipientMode: "all" | "subset" | "single";
  selectedSubset: Record<string, boolean>;
  singleRecipient: string;
  isTaskTag: boolean;
  isWakeTag: boolean;
  wakePolicyGate: boolean;
  replyTo: ReplyTarget | null;
  pendingAttachments: PendingAttachment[];
  setDeliveryNotices: (notices: HarnessDeliveryNotice[]) => void;
  refreshHarnessSessions: () => Promise<void>;
  setMessageInput: (value: string) => void;
  setReplyTo: (value: ReplyTarget | null) => void;
  setPendingAttachments: (value: PendingAttachment[]) => void;
  forceScrollRef: MutableRefObject<boolean>;
  stickToBottomRef: MutableRefObject<boolean>;
  onRefresh: (options?: { includeCapture?: boolean }) => Promise<void>;
}) {
  return async function handleSendMessage() {
    const {
      activeChannel, activeWorkSession, hubAgents, workspacePath, messageInput, sending, setSending,
      recipientMode, selectedSubset, singleRecipient, isTaskTag, isWakeTag, wakePolicyGate, replyTo,
      pendingAttachments, setDeliveryNotices, refreshHarnessSessions, setMessageInput, setReplyTo,
      setPendingAttachments, forceScrollRef, stickToBottomRef, onRefresh,
    } = params;
    if (!messageInput.trim() || sending) return;
    const dmTarget = activeChannel.startsWith("dm-") ? activeChannel.replace("dm-", "") : null;
    if (dmTarget === "human") return;
    setSending(true);
    try {
      const sessionChannel = activeChannel.startsWith("session:") ? activeChannel : null;
      let bodyText = messageInput.trim();
      const enrolledRoster = rosterAgentIds(hubAgents).filter(id => id !== "human" && id !== "system");

      const eligibleRecipients = sessionChannel && activeWorkSession
        ? activeWorkSession.member_ids.filter(id => id !== "human" && id !== "system")
        : enrolledRoster;
      let targetAgents: string[] = [];
      if (dmTarget) {
        targetAgents = [dmTarget];
      } else if (recipientMode === "single") {
        targetAgents = [singleRecipient];
      } else if (recipientMode === "subset") {
        // A subset starts with every eligible member selected. The old UI
        // rendered an absent key as checked but only sent explicitly present
        // keys, so an untouched subset could accidentally address nobody.
        targetAgents = eligibleRecipients.filter(id => selectedSubset[id] !== false);
        if (targetAgents.length === 0) {
          alert("Please select at least one recipient agent for subset messaging.");
          setSending(false);
          return;
        }
      } else {
        targetAgents = eligibleRecipients;
      }

      // C11 Validation: Task-tagged messages MUST target existing team members
      if (isTaskTag) {
        const nonTeamTargets = targetAgents.filter(id => !enrolledRoster.includes(id));
        if (nonTeamTargets.length > 0) {
          alert(`Task-tagged messages must target existing team members. Target(s) not on team: ${nonTeamTargets.join(", ")}. Please enroll the agent or use [WAKE] tag to spawn a new instance.`);
          setSending(false);
          return;
        }
      }

      // Ensure tags are in body text
      if (isTaskTag && !bodyText.startsWith("[TASK]")) {
        bodyText = `[TASK] ${bodyText}`;
      }
      if (isWakeTag && !bodyText.startsWith("[WAKE]")) {
        bodyText = `[WAKE] ${bodyText}`;
      }

      // hub's MessageKind enum only knows message/handoff/wake/system — task
      // intent rides in the `task` field, subject suffix, and [TASK] body
      // prefix instead of a "task" kind, which the backend would reject.
      const messageKind = isWakeTag ? "wake" : "message";
      let subject = dmTarget
        ? `private:${crypto.randomUUID()}`
        : replyTo
          ? `channel:${activeChannel}:thread:${replyTo.id}:${crypto.randomUUID()}`
        : `channel:${activeChannel}:${crypto.randomUUID()}`;

      if (isTaskTag) subject += `:kind:task`;
      else if (isWakeTag) subject += `:kind:wake`;

      const toField = dmTarget
        ? dmTarget
        : recipientMode === "all" && !sessionChannel
          ? "team"
          : targetAgents.join(",");

      if (sessionChannel && activeWorkSession) {
        if (targetAgents.length === 0) throw new Error("The active work session has no members");
        if (isTaskTag || isWakeTag) {
          if (!workspacePath.startsWith("/")) {
            throw new Error("Tagged delivery requires an absolute Workspace Root in Orchestrate");
          }
          setDeliveryNotices(await deliverTaggedSession({
            targetAgents,
            isTaskTag,
            isWakeTag,
            subject,
            bodyText,
            workspacePath,
            sessionId: activeWorkSession.id,
            attachments: pendingAttachments.map(pending => ({
              id: pending.record.id,
              absolutePath: pending.record.absolute_path,
              filename: pending.record.filename,
            })),
          }));
          void refreshHarnessSessions();
        } else {
          await invoke("hub_send_session_message", {
            args: { from: "human", sessionId: activeWorkSession.id, to: targetAgents, subject, workspace: null, task: null, body: bodyText }
          });
        }
      } else if (isTaskTag || isWakeTag) {
        await invoke("hub_send_tagged_message", {
          args: {
            from: "human",
            to: targetAgents,
            isTask: isTaskTag,
            isWake: isWakeTag,
            subject,
            workspace: null,
            task: isTaskTag ? bodyText : null,
            sessionId: null,
            body: bodyText
          }
        });
      } else {
        const sentMsg = await invoke<{ id: string }>("hub_send_message", {
          args: { from: "human", to: toField, kind: messageKind, subject, workspace: null, task: isTaskTag ? bodyText : null, body: bodyText }
        });
        const wakeTargets = toField === "team" ? teamWakeTargets(hubAgents) : targetAgents;
        if (wakePolicyGate) {
          await Promise.all(wakeTargets.map(target => invoke("hub_request_wake", {
            target, reason: `Chat & Memory message in ${activeChannel}`, messageId: sentMsg.id, humanGate: wakePolicyGate
          })));
        }
      }

      setMessageInput("");
      setReplyTo(null);
      pendingAttachments.forEach(pending => URL.revokeObjectURL(pending.previewUrl));
      setPendingAttachments([]);
      forceScrollRef.current = true;
      stickToBottomRef.current = true;
      // Lists only: the 1.5s poll already runs the four-provider capture
      // scan. Awaiting it here kept Send stuck on a full on-disk re-walk.
      await onRefresh({ includeCapture: false });
    } catch (err) {
      alert(`Failed to send message: ${err}`);
    } finally {
      setSending(false);
    }
  };
}
