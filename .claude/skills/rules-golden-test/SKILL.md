---
name: rules-golden-test
description: Use when implementing or changing game rules in Rust (damage, armor ablation, wound states, death save, dice) to write golden tests from worked examples in the Cyberpunk RED book.
---

# Golden tests for rules

Rules live in Rust (`rules-core`, no Tauri dependency). Every rule gets tests derived from the book, not from the implementation.

1. **Find an example.** The book has worked examples with computed answers (sidebars next to the rules; e.g. autofire p.173, melee p.175, cover p.182). Use the `read-rulebook` skill to read them. If a rule has no example, derive the expected value by hand from the rule text and say so in the test name.
2. **Write the test first**: input (damage, location, armor SP, mode, cover HP, ...) → expected outcome (HP lost, new SP, wound state, crit yes/no, cover HP left). Name tests after the rule and book page: `ablation_reduces_sp_only_when_damage_penetrates_p186`.
3. **Dice must be injectable.** Tests use a fixed/scripted RNG; never test randomness itself, test the pipeline given rolls. Every roll supports manual entry.
4. **Cover the edge cases from the book:** armor never stacks (best SP wins); melee/martial arts halve armor rounded up; head + aimed shot doubles only the damage that passed armor; crit (two or more 6s on damage dice) adds 5 damage ignoring armor and an injury not yet present; cover is destroyed at 0 HP and the remainder passes through; death save is d10 vs BODY, 10 always fails, penalty grows by 1 per save.
5. **Return a step log** from the calculator and assert on it for the golden cases ("why X").
6. **Conflicts between book text and tables:** ask the user (AskUserQuestion); record the decision in docs/ROADMAP.md or a code comment with the page number.
7. Run `cargo test` in the crate before finishing.
