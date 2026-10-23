import { useState } from "react";
import { TriangleAlert, X } from "lucide-react";
import type { UseFormReturn } from "react-hook-form";
import { useTranslation } from "react-i18next";
import type { Character } from "@/bindings/Character";
import type { Derived } from "@/bindings/Derived";
import type { Role } from "@/bindings/Role";
import type { RoleEntry } from "@/bindings/RoleEntry";
import { RoleDialog } from "@/components/character/role-dialog";
import { Button } from "@/components/ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { MULTICLASS_MIN_RANK, STARTING_RANK } from "@/lib/character-schema";

type RolesSectionProps = {
  form: UseFormReturn<Character>;
  roles: Role[];
  derived: Derived;
  /** Opens the dose window: this character (a Medtech) gives `drug` to someone. */
  onAdminister: (drug: string) => void;
};

/** Roles of the sheet (multiclass, p. 143): rank and a window for each role's ability. */
export function RolesSection({ form, roles, derived, onAdminister }: RolesSectionProps) {
  const { t } = useTranslation();
  const { watch, setValue } = form;
  const entries = (watch("roles") ?? []) as RoleEntry[];
  const doses = (watch("doses") ?? []) as string[];
  const [openCode, setOpenCode] = useState<string | null>(null);

  const save = (next: RoleEntry[]) => setValue("roles", next, { shouldDirty: true });
  const roleOf = (code: string) => roles.find((r) => r.code === code);
  const drugName = (code: string) =>
    roles
      .flatMap((r) => r.subabilities.flatMap((sub) => sub.options))
      .find((o) => o.code === code)?.name ?? code;
  const available = roles.filter((r) => !entries.some((e) => e.role === r.code));
  // The book lets a new role in once every held one is at rank 4; the GM may allow otherwise.
  const premature = entries.some((e) => e.rank < MULTICLASS_MIN_RANK);
  const open = entries.find((e) => e.role === openCode);
  const openRole = open ? roleOf(open.role) : undefined;

  return (
    <div className="space-y-2">
      {entries.length === 0 && <p className="text-sm text-muted-foreground">{t("roles.none")}</p>}
      <ul className="space-y-1">
        {entries.map((e) => {
          const role = roleOf(e.role);
          return (
            <li key={e.role} className="cut flex items-center gap-2 px-3 py-1.5 [--cut:4px]">
              <span className="flex-1 text-sm">
                <span className="font-semibold">{role?.name ?? e.role}</span>
                {role && <span className="text-muted-foreground"> — {role.ability_name}</span>}
              </span>
              <span className="text-xs text-muted-foreground">{t("roles.rank")}</span>
              <span className="num font-bold">{e.rank}</span>
              <Button type="button" size="xs" variant="outline" onClick={() => setOpenCode(e.role)}>
                {t("roles.open")}
              </Button>
              <Button
                type="button"
                size="icon-xs"
                variant="ghost"
                aria-label={t("roles.remove")}
                onClick={() => save(entries.filter((x) => x.role !== e.role))}
              >
                <X />
              </Button>
            </li>
          );
        })}
      </ul>

      {doses.length > 0 && (
        <div className="flex flex-wrap items-center gap-2 text-xs">
          <span className="text-muted-foreground">{t("drug.doses")}</span>
          {doses.map((d) => (
            <span key={d} className="cut flex items-center gap-1 px-2 py-0.5 [--cut:4px]">
              {drugName(d)}
              <Button
                type="button"
                size="icon-xs"
                variant="ghost"
                aria-label={t("drug.clear")}
                onClick={() => setValue("doses", doses.filter((x) => x !== d), { shouldDirty: true })}
              >
                <X />
              </Button>
            </span>
          ))}
        </div>
      )}

      {available.length > 0 && (
        <div className="flex flex-wrap items-center gap-2">
          <Select
            value=""
            onValueChange={(code) => {
              const rank = entries.length === 0 ? STARTING_RANK : 1;
              save([...entries, { role: code, rank, points: {}, picks: [] }]);
              setOpenCode(code);
            }}
          >
            <SelectTrigger className="w-56" aria-label={t("roles.add")}>
              <SelectValue placeholder={t("roles.add")} />
            </SelectTrigger>
            <SelectContent>
              {available.map((r) => (
                <SelectItem key={r.code} value={r.code}>
                  {r.name} — {r.ability_name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {entries.length > 0 && premature && (
            <span className="flex items-center gap-1 text-xs text-state-light">
              <TriangleAlert className="size-3.5" />
              {t("roles.multiclass_warning", { rank: MULTICLASS_MIN_RANK })}
            </span>
          )}
        </div>
      )}

      {open && openRole && (
        <RoleDialog
          role={openRole}
          entry={open}
          derived={derived}
          onChange={(next) => save(entries.map((e) => (e.role === next.role ? next : e)))}
          onAdminister={(drug) => {
            setOpenCode(null);
            onAdminister(drug);
          }}
          onClose={() => setOpenCode(null)}
        />
      )}
    </div>
  );
}
