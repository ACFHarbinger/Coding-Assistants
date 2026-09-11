import type { TeamMember } from "../components/panels/config/types";

/**
 * Shared team-membership helpers (U22 / #225) so Orchestrate role cards and
 * the Shared Hub roster run the same enroll logic instead of two copies.
 *
 * `App.tsx` rebuilds persisted members from `hub_list_agents` with exactly
 * this shape, and both enroll surfaces map roster rows through
 * `rosterAgentToTeamMember` before calling the same `addAgentToTeam`.
 */
export function rosterAgentToTeamMember(agent: {
  id: string;
  display_name: string;
}): TeamMember {
  return {
    id: agent.id,
    target_id: agent.id,
    name: agent.display_name,
    provider: "",
    model: "",
    origin: "existing",
  };
}

/**
 * Add unless an equal id is already present. Re-enroll while present is a
 * local no-op; the backend `hub_set_team_member` UPDATE is idempotent, so
 * either surface triggering it converges on the same state.
 */
export function addTeamMemberUnique(prev: TeamMember[], next: TeamMember): TeamMember[] {
  if (prev.some((member) => member.id === next.id)) return prev;
  return [...prev, next];
}
