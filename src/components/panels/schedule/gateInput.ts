/** Tokenization only: no shell evaluation, interpolation, or execution. */
export function parseArgv(input: string): string[] {
  const args: string[] = [];
  let token = "";
  let started = false;
  let quote: "'" | '"' | null = null;
  for (let i = 0; i < input.length; i++) {
    const char = input[i];
    if (char === "\\" && quote !== "'") {
      if (i + 1 === input.length) throw new Error("Arguments end with an unfinished escape.");
      token += input[++i];
      started = true;
    } else if (quote) {
      if (char === quote) quote = null;
      else token += char;
    } else if (char === "'" || char === '"') {
      quote = char;
      started = true;
    } else if (/\s/.test(char)) {
      if (started) args.push(token);
      token = "";
      started = false;
    } else {
      token += char;
      started = true;
    }
  }
  if (quote) throw new Error("Arguments contain an unfinished quote.");
  if (started) args.push(token);
  return args;
}

export function gateInteger(value: number | string, minimum: number): number | null {
  if (!String(value).trim()) return null;
  const number = Number(value);
  return Number.isSafeInteger(number) && number >= minimum ? number : null;
}
