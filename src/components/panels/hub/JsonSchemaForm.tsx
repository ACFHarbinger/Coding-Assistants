import { useMemo, type CSSProperties } from "react";

export type JsonSchema = {
  type?: string;
  properties?: Record<string, JsonSchema>;
  required?: string[];
  enum?: Array<string | number | boolean>;
  description?: string;
  default?: unknown;
};

export function emptyArgsFromSchema(schema: JsonSchema | undefined): Record<string, unknown> {
  const properties = schema?.properties ?? {};
  const values: Record<string, unknown> = {};
  for (const [key, field] of Object.entries(properties)) {
    if (field.default !== undefined) {
      values[key] = field.default;
      continue;
    }
    switch (field.type) {
      case "boolean":
        values[key] = false;
        break;
      case "number":
      case "integer":
        values[key] = "";
        break;
      default:
        values[key] = "";
    }
  }
  return values;
}

function FieldInput({
  name,
  field,
  value,
  onChange,
}: {
  name: string;
  field: JsonSchema;
  value: unknown;
  onChange: (next: unknown) => void;
}) {
  if (field.enum && field.enum.length > 0) {
    return (
      <select
        value={String(value ?? "")}
        onChange={(event) => onChange(event.target.value)}
        style={inputStyle}
      >
        <option value="">Select…</option>
        {field.enum.map((option) => (
          <option key={String(option)} value={String(option)}>
            {String(option)}
          </option>
        ))}
      </select>
    );
  }
  if (field.type === "boolean") {
    return (
      <input
        type="checkbox"
        checked={Boolean(value)}
        onChange={(event) => onChange(event.target.checked)}
      />
    );
  }
  if (field.type === "number" || field.type === "integer") {
    return (
      <input
        type="number"
        step={field.type === "integer" ? 1 : "any"}
        value={value === "" || value == null ? "" : String(value)}
        onChange={(event) => onChange(event.target.value === "" ? "" : Number(event.target.value))}
        style={inputStyle}
      />
    );
  }
  return (
    <input
      type="text"
      value={String(value ?? "")}
      onChange={(event) => onChange(event.target.value)}
      placeholder={name}
      style={inputStyle}
    />
  );
}

const inputStyle: CSSProperties = {
  width: "100%",
  padding: "0.6rem 0.75rem",
  borderRadius: "8px",
  background: "rgba(0,0,0,0.4)",
  color: "white",
  border: "1px solid var(--border-color)",
  outline: "none",
};

export function JsonSchemaForm({
  schema,
  value,
  onChange,
}: {
  schema: JsonSchema | undefined;
  value: Record<string, unknown>;
  onChange: (next: Record<string, unknown>) => void;
}) {
  const properties = schema?.properties ?? {};
  const required = useMemo(() => new Set(schema?.required ?? []), [schema]);
  const names = Object.keys(properties);

  if (names.length === 0) {
    return (
      <p style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>
        This tool takes no arguments.
      </p>
    );
  }

  return (
    <div style={{ display: "grid", gap: "0.75rem" }}>
      {names.map((name) => {
        const field = properties[name];
        return (
          <label key={name} style={{ display: "grid", gap: "0.35rem", color: "var(--text-main)" }}>
            <span style={{ fontSize: "0.85rem" }}>
              {name}
              {required.has(name) ? " *" : ""}
              {field.description ? (
                <span style={{ color: "var(--text-muted)", marginLeft: "0.5rem" }}>
                  {field.description}
                </span>
              ) : null}
            </span>
            <FieldInput
              name={name}
              field={field}
              value={value[name]}
              onChange={(next) => onChange({ ...value, [name]: next })}
            />
          </label>
        );
      })}
    </div>
  );
}

export function preparedArguments(
  schema: JsonSchema | undefined,
  value: Record<string, unknown>,
): Record<string, unknown> {
  const required = new Set(schema?.required ?? []);
  const properties = schema?.properties ?? {};
  const out: Record<string, unknown> = {};
  for (const [key, raw] of Object.entries(value)) {
    const field = properties[key];
    if (raw === "" || raw === undefined || raw === null) {
      if (required.has(key)) {
        throw new Error(`Missing required argument: ${key}`);
      }
      continue;
    }
    if (field?.type === "integer" && typeof raw === "number") {
      out[key] = Math.trunc(raw);
      continue;
    }
    out[key] = raw;
  }
  for (const key of required) {
    if (!(key in out)) {
      throw new Error(`Missing required argument: ${key}`);
    }
  }
  return out;
}
