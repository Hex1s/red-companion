---
name: add-ui-component
description: Use when adding or restyling a UI component in this project (shadcn primitive or custom React component). Gives the checklist for the "tactical HUD" design system: i18n, colour tokens, num class, cut corners.
---

# Adding a UI component

Follow the design rules in CLAUDE.md. Checklist:

1. **shadcn primitive?** Run `npx shadcn@4.21.0 add <name>`. If it fails with `EALLOWSCRIPTS`, install the missing package manually with `npm i`. The generated file imports `cn` from the `cn` package; leave that as is.
2. **Restyle the generated file** to the design system:
   - no `rounded-*` (radius is 0), no `shadow-*`;
   - panels/buttons use the `cut` utility; never put `border` or `bg-*` on an element that has `cut` — use `[--cut-border:...]` / `[--cut-fill:...]` instead; never write a custom `clip-path`;
   - headings and buttons are Tektur uppercase (already global for `h1`–`h4`; buttons carry `font-display`).
3. **Colours:** only tokens (`bg-primary`, `text-muted-foreground`, `bg-state-ok` ...). No hex, no `rgb()`, no Tailwind palette (`red-500`). A new colour is added first as a token in `src/index.css` (`:root` and `@theme inline`).
4. **Numbers** (HP, SP, initiative, rolls, stats) always get the `num` class.
5. **Text:** no hard-coded strings in JSX. Add the key to **both** `src/i18n/locales/ru.json` and `en.json`, use `useTranslation()`.
6. **Code style:** kebab-case file name, named PascalCase export, props typed as `type XProps`, no `any`. Put it in `components/ui/` (primitive), `components/layout/`, or a feature folder like `components/character/`.
7. **State:** UI state only (Zustand or local). Game data and rules come from Rust via Tauri commands.
8. **Verify:** `npx tsc --noEmit` must pass; for visual changes take a screenshot (run `npm run dev`, then headless Edge: `msedge --headless --screenshot=out.png --window-size=1280,720 http://localhost:1420`) and look at it.

If something is unclear (new colour meaning, new layout area), ask the user with AskUserQuestion instead of guessing.
