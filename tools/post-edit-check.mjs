// Claude Code PostToolUse hook (see .claude/settings.json): after an edit of a front-end source or a
// locale file, runs `npm run check` (tsc + ru/en key parity). Silent on success; on failure prints the
// output to stderr and exits 2, which hands it back to Claude as feedback.
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";

const MAX_LINES = 40;

let event = {};
try {
  event = JSON.parse(readFileSync(0, "utf8"));
} catch {
  process.exit(0);
}

const file = String(event.tool_input?.file_path ?? "").replaceAll("\\", "/");
const relevant = /\/src\/.+\.(ts|tsx)$/.test(file) || /\/src\/i18n\/locales\/(ru|en)\.json$/.test(file);
if (!relevant) process.exit(0);

const run = spawnSync("npm", ["run", "check", "--silent"], {
  cwd: process.env.CLAUDE_PROJECT_DIR || process.cwd(),
  encoding: "utf8",
  shell: true,
});
if (run.status === 0) process.exit(0);

const lines = `${run.stdout ?? ""}${run.stderr ?? ""}`.split("\n").filter((l) => l.trim() !== "");
console.error(`npm run check failed after editing ${file}:`);
console.error(lines.slice(0, MAX_LINES).join("\n"));
if (lines.length > MAX_LINES) console.error(`… and ${lines.length - MAX_LINES} more lines`);
process.exit(2);
