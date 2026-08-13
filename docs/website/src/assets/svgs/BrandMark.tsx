export function BrandMark({ className = "h-9 w-9", title = "Coding-Assistants" }: {
  className?: string;
  title?: string;
}) {
  return (
    <svg
      className={className}
      viewBox="0 0 48 48"
      fill="none"
      role="img"
      aria-label={title}
    >
      <title>{title}</title>
      <circle cx="20" cy="24" r="13" stroke="#6366f1" strokeWidth="5" />
      <circle cx="28" cy="24" r="13" stroke="#a855f7" strokeWidth="5" />
      <circle cx="24" cy="24" r="4" fill="var(--bg-primary)" stroke="#6366f1" strokeWidth="1.5" />
    </svg>
  );
}
