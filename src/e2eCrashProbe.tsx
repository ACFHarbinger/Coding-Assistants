import { useEffect, useState } from "react";

/**
 * Test-only: lets a WebDriver e2e run force a render throw inside
 * `AppErrorBoundary` to verify #143's crash-recovery view, since the app has
 * no other way to trigger one on demand.
 *
 * Rendered from `main.tsx` only when `import.meta.env.VITE_E2E_CRASH_HOOK` is
 * set at build time. Vite statically replaces that expression, so with the
 * flag unset this component and its import are dead-code-eliminated from the
 * production bundle — there is no runtime hook in a normal build.
 */
export default function E2ECrashProbe() {
  const [boom, setBoom] = useState(false);
  if (boom) {
    throw new Error("E2E forced render crash (VITE_E2E_CRASH_HOOK)");
  }
  useEffect(() => {
    const w = window as unknown as { __E2E_FORCE_RENDER_CRASH__?: () => void };
    w.__E2E_FORCE_RENDER_CRASH__ = () => setBoom(true);
    return () => {
      delete w.__E2E_FORCE_RENDER_CRASH__;
    };
  }, []);
  return null;
}
