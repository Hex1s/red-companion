// Publishes a built app release to the Red Companion server (docs/SERVER-SPEC.md in the server
// repository, 8.2 item 9). Runs in the last job of .github/workflows/release.yml after the build
// jobs have uploaded their files.
//
//   node tools/upload-release.mjs <version> <artifacts dir>
//
// Environment: RC_SERVER_URL (https://red-companion.ru), RC_CI_TOKEN (service token, scope
// releases:write), ANDROID_ENABLED ("true" when an APK is expected), GITHUB_SHA (optional),
// RC_MIN_SUPPORTED (optional X.Y.Z: older apps are told the update is mandatory).
// Steps: read CHANGELOG.md, find the files, declare the release, upload every file (streamed,
// with its SHA-256 and updater signature), publish. Nothing is published unless every file is there.
import { createHash } from "node:crypto";
import { createReadStream, existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import { Readable } from "node:stream";

const [version, artifactsDir] = process.argv.slice(2);
const { RC_SERVER_URL, RC_CI_TOKEN, ANDROID_ENABLED, GITHUB_SHA, RC_MIN_SUPPORTED } = process.env;
if (!/^\d+\.\d+\.\d+$/.test(version ?? "") || !artifactsDir || !RC_SERVER_URL || !RC_CI_TOKEN) {
  console.error("Usage: RC_SERVER_URL=... RC_CI_TOKEN=... node tools/upload-release.mjs <X.Y.Z> <artifacts dir>");
  process.exit(1);
}
const server = RC_SERVER_URL.replace(/\/+$/, "");

function changelogSection(language) {
  const text = readFileSync("CHANGELOG.md", "utf8").replace(/\r\n/g, "\n");
  const versions = text.split(/^## /m).slice(1);
  const section = versions.find((v) => v.split("\n")[0].trim() === version);
  if (!section) throw new Error(`CHANGELOG.md has no section "## ${version}"`);
  const part = section.split(/^### /m).slice(1).find((p) => p.split("\n")[0].trim() === language);
  const body = part?.split("\n").slice(1).join("\n").trim();
  if (!body) throw new Error(`CHANGELOG.md: "## ${version}" has no non-empty "### ${language}"`);
  return body;
}

function walk(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? walk(path.join(dir, e.name)) : [path.join(dir, e.name)]));
}
const found = walk(artifactsDir);

/** One file of the release: where it comes from, how it is declared, its `.sig`. */
const slots = [
  { platform: "windows", arch: "x86_64", kind: "installer", match: /-setup\.exe$/, signed: true, ext: "-setup.exe", dir: "windows" },
  { platform: "macos", arch: "aarch64", kind: "installer", match: /\.dmg$/, signed: false, ext: ".dmg", dir: "macos-aarch64" },
  { platform: "macos", arch: "aarch64", kind: "updater", match: /\.app\.tar\.gz$/, signed: true, ext: ".app.tar.gz", dir: "macos-aarch64" },
  { platform: "macos", arch: "x86_64", kind: "installer", match: /\.dmg$/, signed: false, ext: ".dmg", dir: "macos-x86_64" },
  { platform: "macos", arch: "x86_64", kind: "updater", match: /\.app\.tar\.gz$/, signed: true, ext: ".app.tar.gz", dir: "macos-x86_64" },
];
if (ANDROID_ENABLED === "true") {
  slots.push({ platform: "android", arch: "universal", kind: "apk", match: /\.apk$/, signed: false, ext: ".apk", dir: "android" });
}

const files = slots.map((slot) => {
  const hits = found.filter((f) => f.split(path.sep).includes(`release-${slot.dir}`) && slot.match.test(f));
  if (hits.length !== 1) throw new Error(`Expected exactly one ${slot.dir} file matching ${slot.match}, found ${hits.length}`);
  const archPart = slot.platform === "windows" ? "x64" : slot.arch === "universal" ? "android" : slot.arch;
  const fileName = `Red-Companion_${version}_${archPart}${slot.ext}`;
  const signatureFile = `${hits[0]}.sig`;
  if (slot.signed && !existsSync(signatureFile)) throw new Error(`Missing signature ${signatureFile} (is TAURI_SIGNING_PRIVATE_KEY set?)`);
  return { ...slot, source: hits[0], fileName, signature: slot.signed ? readFileSync(signatureFile, "utf8").trim() : undefined };
});

const sha256 = (file) =>
  new Promise((resolve, reject) => {
    const hash = createHash("sha256");
    createReadStream(file).on("data", (c) => hash.update(c)).on("end", () => resolve(hash.digest("hex"))).on("error", reject);
  });

async function request(method, url, init = {}) {
  const response = await fetch(server + url, { method, ...init, headers: { Authorization: `Bearer ${RC_CI_TOKEN}`, ...init.headers } });
  const text = await response.text();
  if (!response.ok) throw new Error(`${method} ${url} -> ${response.status} ${text}`);
  return text ? JSON.parse(text) : null;
}

const declaration = {
  version,
  changelog: { ru: changelogSection("ru"), en: changelogSection("en") },
  min_supported_version: RC_MIN_SUPPORTED || undefined,
  source_commit: GITHUB_SHA?.slice(0, 40),
  assets: files.map((f) => ({ platform: f.platform, arch: f.arch, kind: f.kind, file_name: f.fileName, signed: f.signed })),
};
const created = await request("POST", "/api/ci/app-releases", { headers: { "Content-Type": "application/json" }, body: JSON.stringify(declaration) });
console.log(`Release ${version} declared (${created.files.length} file(s) already on the server).`);

for (const f of files) {
  const size = statSync(f.source).size;
  const headers = { "Content-Type": "application/octet-stream", "Content-Length": String(size), "X-Asset-Sha256": await sha256(f.source) };
  if (f.signature) headers["X-Asset-Signature"] = f.signature;
  await request("PUT", `/api/ci/app-releases/${version}/assets/${encodeURIComponent(f.fileName)}`, {
    headers,
    body: Readable.toWeb(createReadStream(f.source)),
    duplex: "half",
  });
  console.log(`Uploaded ${f.fileName} (${(size / 1048576).toFixed(1)} MB)`);
}

await request("POST", `/api/ci/app-releases/${version}/publish`);
console.log(`Release ${version} published.`);
