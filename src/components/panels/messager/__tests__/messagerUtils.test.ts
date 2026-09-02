import { describe, expect, it } from "vitest";
import { FALLBACK_ROSTER, rosterAgentIds, uniqueChannelPosts } from "../utils";
import type { HubAgent, HubMessage } from "../types";

describe("rosterAgentIds (#243 QA-6)", () => {
  it("uses FALLBACK_ROSTER when only human is enrolled", () => {
    const agents: HubAgent[] = [
      { id: "human", display_name: "Human Developer", team_member: true },
    ];
    const roster = rosterAgentIds(agents);
    expect(roster).toEqual(FALLBACK_ROSTER);
  });

  it("uses FALLBACK_ROSTER when only human and system are present", () => {
    const agents: HubAgent[] = [
      { id: "human", display_name: "Human Developer", team_member: true },
      { id: "system", display_name: "System", team_member: true },
    ];
    const roster = rosterAgentIds(agents);
    expect(roster).toEqual(FALLBACK_ROSTER);
  });

  it("uses explicit roster when other team members are enrolled", () => {
    const agents: HubAgent[] = [
      { id: "human", display_name: "Human Developer", team_member: true },
      { id: "claude", display_name: "Claude", team_member: true },
      { id: "gemini", display_name: "Gemini", team_member: true },
    ];
    const roster = rosterAgentIds(agents);
    expect(roster).toEqual(["human", "claude", "gemini"]);
  });
});

describe("uniqueChannelPosts recipient aggregation (#245 QA-8)", () => {
  it("aggregates fan-out message rows into a single post with all recipients", () => {
    const now = "2026-09-02T11:00:00Z";
    const fanoutMessages: HubMessage[] = [
      {
        id: "msg-1",
        from_agent: "human",
        to_agent: "chat",
        body: "Team sync starting",
        subject: "channel:general",
        kind: "plain",
        status: "delivered",
        created_at: now,
      },
      {
        id: "msg-2",
        from_agent: "human",
        to_agent: "claude",
        body: "Team sync starting",
        subject: "channel:general",
        kind: "plain",
        status: "delivered",
        created_at: now,
      },
      {
        id: "msg-3",
        from_agent: "human",
        to_agent: "gemini",
        body: "Team sync starting",
        subject: "channel:general",
        kind: "plain",
        status: "delivered",
        created_at: now,
      },
      {
        id: "msg-4",
        from_agent: "human",
        to_agent: "grok",
        body: "Team sync starting",
        subject: "channel:general",
        kind: "plain",
        status: "delivered",
        created_at: now,
      },
    ];

    const posts = uniqueChannelPosts(fanoutMessages, "general");
    expect(posts).toHaveLength(1);
    expect(posts[0].recipient_agents).toEqual(["chat", "claude", "gemini", "grok"]);
  });

  it("handles comma-separated to_agent recipients correctly", () => {
    const now = "2026-09-02T11:00:00Z";
    const messages: HubMessage[] = [
      {
        id: "msg-1",
        from_agent: "human",
        to_agent: "chat, claude",
        body: "Subset announcement",
        subject: "channel:general",
        kind: "plain",
        status: "delivered",
        created_at: now,
      },
    ];

    const posts = uniqueChannelPosts(messages, "general");
    expect(posts).toHaveLength(1);
    expect(posts[0].recipient_agents).toEqual(["chat", "claude"]);
  });
});
