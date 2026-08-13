import { Moon, Sun, Monitor } from "lucide-react";
import { useTheme } from "../app/ThemeProvider";

const OPTIONS = [
  { id: "dark" as const, label: "Dark", icon: Moon },
  { id: "light" as const, label: "Light", icon: Sun },
  { id: "system" as const, label: "System", icon: Monitor },
];

export function ThemeToggle() {
  const { theme, setTheme } = useTheme();

  return (
    <div
      role="radiogroup"
      aria-label="Color theme"
      className="inline-flex rounded-lg border border-white/10 bg-slate-950/70 p-0.5"
    >
      {OPTIONS.map((option) => {
        const Icon = option.icon;
        const selected = theme === option.id;
        return (
          <button
            key={option.id}
            type="button"
            role="radio"
            aria-checked={selected}
            onClick={() => setTheme(option.id)}
            className={`inline-flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-400 ${
              selected
                ? "bg-indigo-500/20 text-indigo-200"
                : "text-slate-400 hover:text-slate-100"
            }`}
          >
            <Icon className="h-3.5 w-3.5" aria-hidden="true" />
            <span className="hidden sm:inline">{option.label}</span>
          </button>
        );
      })}
    </div>
  );
}
