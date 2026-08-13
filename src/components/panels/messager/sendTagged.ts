import { invoke } from "../../../lib/tauri";
import type { HarnessDeliveryNotice } from "../harness/types";
import { injectNotice, isSuccessfulInject } from "../harness/types";
import type { HarnessInjectResult, TaggedSendOutcome } from "./types";

export async function deliverTaggedSession(args: {
  targetAgents: string[];
  isTaskTag: boolean;
  isWakeTag: boolean;
  subject: string;
  bodyText: string;
  workspacePath: string;
  sessionId: string;
}): Promise<HarnessDeliveryNotice[]> {
  const outcomes = await invoke<TaggedSendOutcome[]>("hub_send_tagged_message", {
    args: {
      from: "human",
      to: args.targetAgents,
      isTask: args.isTaskTag,
      isWake: args.isWakeTag,
      subject: args.subject,
      workspace: null,
      task: args.isTaskTag ? args.bodyText : null,
      sessionId: args.sessionId,
      body: args.bodyText,
    },
  });
  const accepted = outcomes.filter((outcome) => outcome.accepted && outcome.message_id);
  const injections = await Promise.allSettled(
    accepted.map((outcome) => invoke<HarnessInjectResult>("hub_inject_harness", {
      harness: outcome.to_agent,
      workspace: args.workspacePath,
      sessionId: args.sessionId,
      messageId: outcome.message_id,
      body: args.bodyText,
      isTask: args.isTaskTag,
      isWake: args.isWakeTag,
    })),
  );
  const notices: HarnessDeliveryNotice[] = [
    ...outcomes.filter((outcome) => !outcome.accepted).map((outcome) => ({
      harness: outcome.to_agent,
      status: "unavailable",
      detail: outcome.reason || "rejected",
      retryable: false,
    })),
    ...injections.map((result, index) => {
      const target = accepted[index];
      if (result.status === "rejected") {
        return {
          harness: target?.to_agent ?? "harness",
          status: "unavailable",
          detail: String(result.reason),
          retryable: true,
          messageId: target?.message_id,
          body: args.bodyText,
          isTask: args.isTaskTag,
          isWake: args.isWakeTag,
        };
      }
      const notice = injectNotice(result.value.status, result.value.detail);
      return {
        harness: result.value.harness,
        status: result.value.status,
        detail: result.value.detail,
        retryable: notice.retryable,
        messageId: target?.message_id,
        body: args.bodyText,
        isTask: args.isTaskTag,
        isWake: args.isWakeTag,
      };
    }),
  ];
  return notices.filter((notice) => !isSuccessfulInject(notice.status));
}
