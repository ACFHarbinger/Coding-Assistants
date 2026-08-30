import { useEffect, useState } from "react";
import type {
  EffectiveHarnessSettings,
  EffectiveSettings,
  HarnessModelCatalog,
  HarnessSettings,
  ProfileSnapshot,
  ProviderProfile,
} from "../types";
import {
  getAllHarnessOptions,
  listSettingsHarnesses,
  listSettingsProfiles,
  removeSettingsProfile,
  resetWorkspaceDefaultProfile,
  resetWorkspaceHarnessEffort,
  resetWorkspaceHarnessModel,
  setHarnessEffort,
  setHarnessModel,
  setWorkspaceDefaultProfile,
  setWorkspaceHarnessEffort,
  setWorkspaceHarnessModel,
  updateSettingsHarness,
  upsertSettingsProfile,
} from "../api";
import { shortenPath } from "./shared";
import { HarnessCard } from "./agents/HarnessCard";
import { ProfileSection } from "./agents/ProfileSection";

export interface AgentsTabProps {
  effective: EffectiveSettings;
  scope: "global" | "workspace";
  setScope: (scope: "global" | "workspace") => void;
  workspaceRoot: string | null;
  busy: boolean;
  onChanged: () => void;
}

export default function AgentsTab({
  effective,
  scope,
  setScope,
  workspaceRoot,
  busy,
  onChanged,
}: AgentsTabProps) {
  const [profiles, setProfiles] = useState<ProfileSnapshot[]>(effective.profiles ?? []);
  const [harnesses, setHarnesses] = useState<EffectiveHarnessSettings[]>(effective.harnesses ?? []);
  const [catalogs, setCatalogs] = useState<Record<string, HarnessModelCatalog>>({});
  const [profileError, setProfileError] = useState<string | null>(null);

  const targetWorkspace = scope === "workspace" ? workspaceRoot : null;

  const refreshProfilesAndHarnesses = async () => {
    try {
      const [pList, hList, cMap] = await Promise.all([
        listSettingsProfiles(),
        listSettingsHarnesses(targetWorkspace),
        getAllHarnessOptions(),
      ]);
      setProfiles(pList);
      setHarnesses(hList);
      setCatalogs(cMap);
    } catch {
      // Non-critical background refresh failure
    }
  };

  useEffect(() => {
    void refreshProfilesAndHarnesses();
  }, [targetWorkspace]);

  const handleSaveProfile = async (payload: ProviderProfile) => {
    try {
      const updated = await upsertSettingsProfile(payload);
      setProfiles(updated);
      setProfileError(null);
      onChanged();
    } catch (err) {
      setProfileError(String(err));
    }
  };

  const handleRemoveProfile = async (name: string) => {
    try {
      const updated = await removeSettingsProfile(name);
      setProfiles(updated);
      onChanged();
    } catch (err) {
      setProfileError(String(err));
    }
  };

  const handleSelectDefaultProfile = async (harness: string, profileName: string) => {
    if (!workspaceRoot) return;
    try {
      if (profileName) {
        await setWorkspaceDefaultProfile(workspaceRoot, harness, profileName);
      } else {
        await resetWorkspaceDefaultProfile(workspaceRoot, harness);
      }
      await refreshProfilesAndHarnesses();
      onChanged();
    } catch (err) {
      setProfileError(String(err));
    }
  };

  const handleSelectModel = async (harness: string, model: string) => {
    try {
      if (scope === "workspace" && workspaceRoot) {
        await setWorkspaceHarnessModel(workspaceRoot, harness, model);
      } else {
        await setHarnessModel(harness, model || null);
      }
      await refreshProfilesAndHarnesses();
      onChanged();
    } catch (err) {
      setProfileError(String(err));
    }
  };

  const handleResetModel = async (harness: string) => {
    if (!workspaceRoot) return;
    try {
      await resetWorkspaceHarnessModel(workspaceRoot, harness);
      await refreshProfilesAndHarnesses();
      onChanged();
    } catch (err) {
      setProfileError(String(err));
    }
  };

  const handleSelectEffort = async (harness: string, effort: string) => {
    try {
      if (scope === "workspace" && workspaceRoot) {
        await setWorkspaceHarnessEffort(workspaceRoot, harness, effort);
      } else {
        await setHarnessEffort(harness, effort || null);
      }
      await refreshProfilesAndHarnesses();
      onChanged();
    } catch (err) {
      setProfileError(String(err));
    }
  };

  const handleResetEffort = async (harness: string) => {
    if (!workspaceRoot) return;
    try {
      await resetWorkspaceHarnessEffort(workspaceRoot, harness);
      await refreshProfilesAndHarnesses();
      onChanged();
    } catch (err) {
      setProfileError(String(err));
    }
  };

  const handleToggleHarnessField = async (
    h: EffectiveHarnessSettings,
    field: "capture_polling" | "inject_permission",
  ) => {
    const updated: HarnessSettings = {
      harness: h.harness,
      executable: h.executable,
      workdir: h.workdir,
      capture_polling: field === "capture_polling" ? !h.capture_polling : h.capture_polling,
      inject_permission: field === "inject_permission" ? !h.inject_permission : h.inject_permission,
    };
    try {
      await updateSettingsHarness(updated);
      await refreshProfilesAndHarnesses();
      onChanged();
    } catch (err) {
      setProfileError(String(err));
    }
  };

  return (
    <div>
      {workspaceRoot && (
        <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1.25rem" }}>
          <button
            type="button"
            className={scope === "workspace" ? "btn-primary" : "btn-secondary"}
            style={{ marginTop: 0, padding: "0.35rem 0.8rem", fontSize: "0.8rem" }}
            onClick={() => setScope("workspace")}
          >
            Workspace ({shortenPath(workspaceRoot, 28)})
          </button>
          <button
            type="button"
            className={scope === "global" ? "btn-primary" : "btn-secondary"}
            style={{ marginTop: 0, padding: "0.35rem 0.8rem", fontSize: "0.8rem" }}
            onClick={() => setScope("global")}
          >
            Global defaults
          </button>
        </div>
      )}

      {profileError && (
        <div
          style={{
            padding: "0.6rem 0.85rem",
            borderRadius: "8px",
            background: "rgba(239, 68, 68, 0.12)",
            border: "1px solid rgba(248, 113, 113, 0.45)",
            color: "#fca5a5",
            fontSize: "0.82rem",
            marginBottom: "1rem",
          }}
        >
          {profileError}
        </div>
      )}

      {/* Profiles Section */}
      <ProfileSection
        profiles={profiles}
        onSaveProfile={handleSaveProfile}
        onRemoveProfile={handleRemoveProfile}
        setProfileError={setProfileError}
      />

      {/* Harnesses Section */}
      <section style={{ marginBottom: "1.75rem" }}>
        <h3 style={{ margin: "0 0 0.4rem", fontSize: "0.95rem", fontWeight: 700 }}>
          Harness Models, Defaults & Runtime Policy
        </h3>
        <p style={{ color: "var(--text-muted)", fontSize: "0.8rem", margin: "0 0 0.85rem" }}>
          Per-harness model selection, reasoning effort levels, and task injection permissions.
        </p>

        <div style={{ display: "grid", gap: "0.75rem" }}>
          {harnesses.map((h) => (
            <HarnessCard
              key={h.harness}
              harness={h}
              catalog={catalogs[h.harness]}
              profiles={profiles}
              scope={scope}
              workspaceRoot={workspaceRoot}
              busy={busy}
              onSelectProfile={handleSelectDefaultProfile}
              onSelectModel={handleSelectModel}
              onResetModel={handleResetModel}
              onSelectEffort={handleSelectEffort}
              onResetEffort={handleResetEffort}
              onToggleField={handleToggleHarnessField}
            />
          ))}
        </div>
      </section>
    </div>
  );
}
