import { describe, expect, it } from "vitest";
import { addTeamMemberUnique, rosterAgentToTeamMember } from "../team";
import type { TeamMember } from "../../components/panels/config/types";

const member = (id: string, target_id: string = id): TeamMember => ({
  id,
  target_id,
  name: id,
  provider: "",
  model: "",
  origin: "existing",
});

describe("team membership helpers (U22 / #225)", () => {
  it("maps a roster row to the TeamMember shape App persists", () => {
    expect(rosterAgentToTeamMember({ id: "kimi", display_name: "Kimi" })).toEqual({
      id: "kimi",
      target_id: "kimi",
      name: "Kimi",
      provider: "",
      model: "",
      origin: "existing",
    });
  });

  it("adds a new member and ignores duplicate ids", () => {
    const prev = [member("claude")];
    const added = addTeamMemberUnique(prev, member("kimi"));
    expect(added.map((m) => m.id)).toEqual(["claude", "kimi"]);

    // Re-enroll while present: same array back, no duplicate row.
    expect(addTeamMemberUnique(added, member("kimi"))).toBe(added);
  });

  it("keeps role-spawned transients distinct from roster ids", () => {
    // Orchestrate role cards use `role:`/`process:` target ids; the guard
    // keys on member id, so a roster enroll never collides with them.
    const prev = [member("role:planner", "role:planner")];
    const added = addTeamMemberUnique(prev, member("kimi"));
    expect(added).toHaveLength(2);
  });
});
