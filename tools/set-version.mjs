// Sets the app version in every place the release pipeline compares (release.yml, job `check`):
//   node tools/set-version.mjs 0.9.0        (or: npm run version:set -- 0.9.0)
// Patches package.json, package-lock.json, src-tauri/tauri.conf.json, [workspace.package] in Cargo.toml
// and the workspace crates in Cargo.lock; adds a CHANGELOG.md section to fill in if it is missing.
// It does not touch git: committing and tagging stay with the owner (the commands are printed).
import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const version = (process.argv[2] ?? "").replace(/^v/, "");
if (!/^\d+\.\d+\.\d+$/.test(version)) {
  console.error("usage: node tools/set-version.mjs X.Y.Z (no pre-release suffix: the server rejects it)");
  process.exit(2);
}

const read = (path) => readFileSync(join(root, path), "utf8");
const changed = [];
function patch(path, edit) {
  const before = read(path);
  const after = edit(before);
  if (after === before) return;
  writeFileSync(join(root, path), after);
  changed.push(path);
}
/** Replaces exactly one match, so a changed file layout fails loudly instead of silently skipping. */
function replaceOnce(text, pattern, replacement, what) {
  const hits = text.match(new RegExp(pattern.source, pattern.flags.replace("g", "") + "g"));
  if (!hits || hits.length !== 1) throw new Error(`${what}: expected 1 match, found ${hits?.length ?? 0}`);
  return text.replace(pattern, replacement);
}

patch("package.json", (t) => replaceOnce(t, /^(  "version": ")[^"]+(")/m, `$1${version}$2`, "package.json version"));

patch("package-lock.json", (t) => {
  // The first "version" is the package itself, the second one sits under packages[""].
  let n = 0;
  return t.replace(/^(\s*"version": ")[^"]+(")/gm, (m, a, b) => (n++ < 2 ? `${a}${version}${b}` : m));
});

patch("src-tauri/tauri.conf.json", (t) => replaceOnce(t, /^(  "version": ")[^"]+(")/m, `$1${version}$2`, "tauri.conf.json version"));

patch("Cargo.toml", (t) =>
  replaceOnce(t, /(\[workspace\.package\][^[]*?\nversion\s*=\s*")[^"]+(")/, `$1${version}$2`, "Cargo.toml workspace version"),
);

// Cargo.lock lists the workspace crates with the workspace version; keep it in step so a build
// does not rewrite the lock file.
const manifests = ["src-tauri/Cargo.toml", ...readdirSync(join(root, "crates")).map((d) => `crates/${d}/Cargo.toml`)];
const members = manifests
  .filter((p) => existsSync(join(root, p)))
  .map(read)
  .filter((t) => /^version\.workspace\s*=\s*true/m.test(t))
  .map((t) => t.match(/^name\s*=\s*"([^"]+)"/m)?.[1])
  .filter(Boolean);
if (existsSync(join(root, "Cargo.lock"))) {
  patch("Cargo.lock", (t) =>
    t.replace(/\[\[package\]\]\nname = "([^"]+)"\nversion = "[^"]+"\n(?!source)/g, (block, name) =>
      members.includes(name) ? block.replace(/version = "[^"]+"/, `version = "${version}"`) : block,
    ),
  );
}

patch("CHANGELOG.md", (t) => {
  if (new RegExp(`^## ${version.replaceAll(".", "\\.")}$`, "m").test(t)) return t;
  const section = `## ${version}\n\n### ru\n\n- TODO\n\n### en\n\n- TODO\n\n`;
  const first = t.search(/^## /m);
  return first === -1 ? `${t.trimEnd()}\n\n${section}` : t.slice(0, first) + section + t.slice(first);
});

console.log(changed.length ? `Version ${version} set in:\n${changed.map((p) => `  ${p}`).join("\n")}` : `Everything already says ${version}.`);
if (/^- TODO$/m.test(read("CHANGELOG.md").split(/^## /m).find((s) => s.startsWith(`${version}\n`)) ?? "")) {
  console.log(`\n!! Fill in CHANGELOG.md section ${version} (ru and en): the release pipeline refuses a section with TODO.`);
}
console.log(`\nNext (you run):\n  git add -A && git commit -m "chore(release): ${version}"\n  git tag v${version} && git push origin main v${version}`);
