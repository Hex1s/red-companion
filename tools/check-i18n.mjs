// Fails when ru.json and en.json have different key sets.
import { readFileSync } from "node:fs";

const load = (lang) =>
  JSON.parse(readFileSync(new URL(`../src/i18n/locales/${lang}.json`, import.meta.url), "utf8"));

const flatten = (obj, prefix = "") =>
  Object.entries(obj).flatMap(([k, v]) =>
    v !== null && typeof v === "object" ? flatten(v, `${prefix}${k}.`) : [`${prefix}${k}`],
  );

const ru = new Set(flatten(load("ru")));
const en = new Set(flatten(load("en")));
const onlyRu = [...ru].filter((k) => !en.has(k));
const onlyEn = [...en].filter((k) => !ru.has(k));

if (onlyRu.length || onlyEn.length) {
  for (const k of onlyRu) console.error(`missing in en.json: ${k}`);
  for (const k of onlyEn) console.error(`missing in ru.json: ${k}`);
  process.exit(1);
}
console.log(`i18n OK: ${ru.size} keys in both locales`);
