import { Minus, Plus, Syringe, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { Derived } from "@/bindings/Derived";
import type { Role } from "@/bindings/Role";
import type { RoleEntry } from "@/bindings/RoleEntry";
import type { SubAbility } from "@/bindings/SubAbility";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { NumberInput } from "@/components/ui/number-input";
import { Textarea } from "@/components/ui/textarea";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import type { RoleCounter } from "@/bindings/RoleCounter";
import { COUNTER_NAME_MAX, RANK_MAX, ROLE_COUNTERS_MAX, ROLE_NOTES_MAX } from "@/lib/character-schema";
import { cn } from "@/lib/utils";

type RoleDialogProps = {
  role: Role;
  entry: RoleEntry;
  derived: Derived;
  onChange: (entry: RoleEntry) => void;
  /** Gives a picked drug to someone: the sheet closes this window and opens the dose window. */
  onAdminister: (drug: string) => void;
  onClose: () => void;
};

const spentOf = (entry: RoleEntry) => Object.values(entry.points).reduce((a, b) => a + b, 0);

/** Picks that belong to one sub-ability (its options). */
const picksOf = (entry: RoleEntry, sub: SubAbility) =>
  entry.picks.filter((p) => sub.options.some((o) => o.code === p));

/**
 * Window of one role the character holds: rank, where the rank points went, the picks made on
 * the way (drugs...) and what the points have unlocked. Edits go straight to the sheet.
 */
export function RoleDialog({ role, entry, derived, onChange, onAdminister, onClose }: RoleDialogProps) {
  const { t } = useTranslation();
  const free = entry.rank * role.points_per_rank - spentOf(entry);

  const setPoints = (sub: SubAbility, points: number) => {
    const next = { ...entry.points, [sub.code]: points };
    if (points === 0) delete next[sub.code];
    onChange({ ...entry, points: next });
  };

  const togglePick = (code: string, on: boolean) =>
    onChange({
      ...entry,
      picks: on ? [...entry.picks, code] : entry.picks.filter((p) => p !== code),
    });

  const medtech = role.code === "medtech";

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-xl">
        <div className="space-y-1">
          <DialogTitle className="text-xl font-bold">
            {role.name} — {role.ability_name}
          </DialogTitle>
          <DialogDescription>{role.description || role.summary}</DialogDescription>
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <span className="text-sm text-muted-foreground">{t("roles.rank")}</span>
          <Button
            type="button"
            size="icon-xs"
            variant="outline"
            aria-label={t("roles.rank_down")}
            disabled={entry.rank <= 0 || free < role.points_per_rank}
            onClick={() => onChange({ ...entry, rank: entry.rank - 1 })}
          >
            <Minus />
          </Button>
          <span className="num text-lg font-bold">{entry.rank}</span>
          <Button
            type="button"
            size="sm"
            variant="outline"
            disabled={entry.rank >= RANK_MAX}
            onClick={() => onChange({ ...entry, rank: entry.rank + 1 })}
          >
            {t("roles.rank_up")}
          </Button>
          <span className={cn("text-sm", free > 0 ? "text-emphasis" : "text-muted-foreground")}>
            {t("roles.free_points", { count: free })}
          </span>
        </div>

        {role.code === "solo" && (
          <section className="cut space-y-2 p-3 [--cut:6px]">
            <label className="flex items-center gap-2 text-sm">
              <Checkbox
                checked={entry.apply_effects}
                onCheckedChange={(v) => onChange({ ...entry, apply_effects: v === true })}
              />
              {t("roles.apply_effects")}
            </label>
            <p className="text-xs text-muted-foreground">{t("roles.solo_effects_hint")}</p>
            {entry.apply_effects && (
              <ul className="num space-y-0.5 text-xs">
                <li>{t("solo.initiative", { value: derived.solo_initiative })}</li>
                <li>{t("solo.deflection", { value: derived.solo_deflection })}</li>
                <li>{t("solo.perception", { value: derived.solo_perception })}</li>
                <li>{t("solo.precision", { value: derived.solo_precision })}</li>
                <li>{t("solo.spot_weakness", { value: derived.solo_spot_weakness })}</li>
              </ul>
            )}
          </section>
        )}

        {role.ranks.length > 0 && (
          <ol className="space-y-1">
            {role.ranks.map((r) => (
              <li
                key={r.rank}
                className={cn(
                  "flex gap-2 text-sm",
                  r.rank <= entry.rank ? "text-foreground" : "text-muted-foreground",
                )}
              >
                <span className={cn("num w-5 shrink-0", r.rank <= entry.rank && "text-emphasis")}>
                  {r.rank}
                </span>
                <span>{r.summary}</span>
              </li>
            ))}
          </ol>
        )}

        {role.subabilities.map((sub) => {
          const points = entry.points[sub.code] ?? 0;
          const picks = picksOf(entry, sub);
          // One point per rank at most goes to the same part, whatever the pool size.
          const max = Math.min(sub.max_points ?? RANK_MAX, entry.rank);
          return (
            <section key={sub.code} className="cut space-y-2 p-3 [--cut:6px]">
              <div className="flex items-center gap-2">
                <h4 className="flex-1 text-sm">{sub.name}</h4>
                <Button
                  type="button"
                  size="icon-xs"
                  variant="outline"
                  aria-label={t("roles.point_remove", { name: sub.name })}
                  disabled={points <= 0 || picks.length >= points}
                  onClick={() => setPoints(sub, points - 1)}
                >
                  <Minus />
                </Button>
                <span className="num w-10 text-center font-bold">
                  {points}
                  {max < RANK_MAX && (
                    <span className="text-xs text-muted-foreground"> / {max}</span>
                  )}
                </span>
                <Button
                  type="button"
                  size="icon-xs"
                  variant="outline"
                  aria-label={t("roles.point_add", { name: sub.name })}
                  disabled={free <= 0 || points >= max}
                  onClick={() => setPoints(sub, points + 1)}
                >
                  <Plus />
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">{sub.summary}</p>

              {sub.options.length > 0 && (
                <div className="space-y-1">
                  <div className="text-xs text-muted-foreground">
                    {t("roles.picks", { used: picks.length, total: points })}
                    {derived.medical_tech_skill > 0 &&
                      ` · ${t("roles.synthesis", { doses: derived.medical_tech_skill })}`}
                  </div>
                  <ul className="space-y-1">
                    {sub.options.map((o) => {
                      const on = entry.picks.includes(o.code);
                      const locked = !on && picks.length >= points;
                      return (
                        <li key={o.code} className="flex items-start gap-2 text-sm">
                          <Checkbox
                            id={`${role.code}.${o.code}`}
                            checked={on}
                            disabled={locked}
                            onCheckedChange={(v) => togglePick(o.code, v === true)}
                            className="mt-0.5"
                          />
                          <label
                            htmlFor={`${role.code}.${o.code}`}
                            className={cn(locked && "text-muted-foreground")}
                          >
                            <span className="font-semibold">{o.name}</span>
                            <span className="text-muted-foreground"> — {o.summary}</span>
                          </label>
                          {on && (
                            <Button
                              type="button"
                              size="xs"
                              variant="outline"
                              className="ml-auto shrink-0"
                              onClick={() => onAdminister(o.code)}
                            >
                              <Syringe />
                              {t("drug.give")}
                            </Button>
                          )}
                        </li>
                      );
                    })}
                  </ul>
                </div>
              )}

              {sub.levels.length > 0 && (
                <ol className="space-y-1">
                  {sub.levels.map((l) => (
                    <li
                      key={l.points}
                      className={cn(
                        "flex gap-2 text-sm",
                        points >= l.points ? "text-foreground" : "text-muted-foreground",
                      )}
                    >
                      <span className="num w-4 shrink-0 text-emphasis">{l.points}</span>
                      <span>{l.summary}</span>
                    </li>
                  ))}
                </ol>
              )}
            </section>
          );
        })}

        <section className="cut space-y-2 p-3 [--cut:6px]">
          <div className="flex items-center gap-2">
            <h4 className="flex-1 text-sm">{t("roles.counters")}</h4>
            <Button
              type="button"
              size="xs"
              variant="outline"
              disabled={entry.counters.length >= ROLE_COUNTERS_MAX}
              onClick={() =>
                onChange({ ...entry, counters: [...entry.counters, { name: "", value: 0, max: null }] })
              }
            >
              <Plus />
              {t("roles.counter_add")}
            </Button>
          </div>
          {entry.counters.map((c, i) => {
            const set = (patch: Partial<RoleCounter>) =>
              onChange({
                ...entry,
                counters: entry.counters.map((o, j) => (j === i ? { ...o, ...patch } : o)),
              });
            const num = (v: string) => (Number.isFinite(Number(v)) && v !== "" ? Math.trunc(Number(v)) : null);
            return (
              <div key={i} className="flex items-center gap-2">
                <Input
                  value={c.name}
                  placeholder={t("roles.counter_name")}
                  maxLength={COUNTER_NAME_MAX}
                  onChange={(e) => set({ name: e.target.value })}
                  className="flex-1"
                />
                <NumberInput
                  value={c.value}
                  aria-label={t("roles.counter_value")}
                  onChange={(e) => set({ value: num(e.target.value) ?? 0 })}
                  className="num w-16"
                />
                <span className="text-muted-foreground">/</span>
                <NumberInput
                  value={c.max ?? ""}
                  placeholder="∞"
                  aria-label={t("roles.counter_max")}
                  onChange={(e) => set({ max: num(e.target.value) })}
                  className="num w-16"
                />
                <Button
                  type="button"
                  size="icon-xs"
                  variant="outline"
                  aria-label={t("roles.counter_remove")}
                  onClick={() =>
                    onChange({ ...entry, counters: entry.counters.filter((_, j) => j !== i) })
                  }
                >
                  <X />
                </Button>
              </div>
            );
          })}
          <Textarea
            value={entry.notes}
            placeholder={t("roles.notes")}
            maxLength={ROLE_NOTES_MAX}
            rows={3}
            onChange={(e) => onChange({ ...entry, notes: e.target.value })}
          />
        </section>

        {medtech && (
          <div className="grid grid-cols-2 gap-2 text-sm">
            <div className="cut p-2 [--cut:6px]">
              <div className="text-xs text-muted-foreground">{t("roles.skill_surgery")}</div>
              <div className="num text-lg font-bold">{derived.surgery_skill}</div>
            </div>
            <div className="cut p-2 [--cut:6px]">
              <div className="text-xs text-muted-foreground">{t("roles.skill_med_tech")}</div>
              <div className="num text-lg font-bold">{derived.medical_tech_skill}</div>
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
