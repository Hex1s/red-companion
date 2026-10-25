import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ReferenceDetail } from "@/components/reference/reference-detail";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useReferenceData } from "@/hooks/use-reference-data";
import { hasBackend } from "@/lib/api";
import { entriesOf, fold, REFERENCE_SECTIONS } from "@/lib/reference-entries";
import { cn } from "@/lib/utils";
import { useUiStore } from "@/store/ui-store";
import type { WeaponKind } from "@/bindings/WeaponKind";

const WEAPON_KINDS: WeaponKind[] = ["melee", "ranged", "exotic"];

/** Reference: a compact list on the left, the typed card of the chosen entry on the right. */
export function ReferencePage() {
  const { t } = useTranslation();
  const { referenceSection, setReferenceSection, referenceQuery, setReferenceQuery, referenceId, setReferenceId } =
    useUiStore();
  const data = useReferenceData();
  // Section filters; they are plain view state and reset with the page.
  const [weaponKind, setWeaponKind] = useState<WeaponKind | null>(null);
  const [onlyConcealable, setOnlyConcealable] = useState(false);
  const [noPenalty, setNoPenalty] = useState(false);
  const [minSp, setMinSp] = useState("");

  const entries = useMemo(() => entriesOf(referenceSection, data, t), [data, referenceSection, t]);

  const query = fold(referenceQuery.trim());
  const spFrom = Number(minSp) || 0;
  const shown = entries.filter((e) => {
    if (query && !e.search.includes(query)) return false;
    if (e.section === "weapons") {
      if (weaponKind && e.item.kind !== weaponKind) return false;
      if (onlyConcealable && !e.item.concealable) return false;
    }
    if (e.section === "armor") {
      if (noPenalty && e.item.penalty > 0) return false;
      if (e.item.sp < spFrom) return false;
    }
    return true;
  });
  // The chosen entry stays while it is in the list; otherwise the first one is shown.
  const current = shown.find((e) => e.id === referenceId) ?? shown[0] ?? null;

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <h2 className="text-2xl font-bold">{t("reference.title")}</h2>
      <Tabs value={referenceSection} onValueChange={(v) => setReferenceSection(v as typeof referenceSection)}>
        <TabsList className="h-auto flex-wrap justify-start">
          {REFERENCE_SECTIONS.map((s) => (
            <TabsTrigger key={s} value={s}>
              {t(`reference.section.${s}`)}
            </TabsTrigger>
          ))}
        </TabsList>
      </Tabs>
      {!hasBackend ? (
        <p className="text-muted-foreground">{t("characters.no_backend")}</p>
      ) : (
        <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,18rem)_minmax(0,1fr)] gap-4">
          <div className="flex min-h-0 flex-col gap-2">
            <Input
              value={referenceQuery}
              onChange={(e) => setReferenceQuery(e.target.value)}
              placeholder={t("reference.search")}
              aria-label={t("reference.search")}
            />
            {referenceSection === "weapons" && (
              <div className="flex flex-wrap items-center gap-1.5">
                {[null, ...WEAPON_KINDS].map((k) => (
                  <Button
                    key={k ?? "all"}
                    size="xs"
                    variant={weaponKind === k ? "default" : "outline"}
                    onClick={() => setWeaponKind(k)}
                  >
                    {t(k ? `reference.weapon.kind.${k}` : "reference.filter.all")}
                  </Button>
                ))}
                <label className="flex items-center gap-2 text-xs whitespace-nowrap">
                  <Checkbox checked={onlyConcealable} onCheckedChange={(v) => setOnlyConcealable(v === true)} />
                  {t("reference.filter.concealable")}
                </label>
              </div>
            )}
            {referenceSection === "armor" && (
              <div className="flex flex-wrap items-center gap-3">
                <label className="flex items-center gap-2 text-xs whitespace-nowrap">
                  <Checkbox checked={noPenalty} onCheckedChange={(v) => setNoPenalty(v === true)} />
                  {t("reference.filter.no_penalty")}
                </label>
                <label className="flex items-center gap-2 text-xs whitespace-nowrap">
                  {t("reference.filter.sp_from")}
                  <Input
                    className="num h-7 w-14"
                    inputMode="numeric"
                    value={minSp}
                    onChange={(e) => setMinSp(e.target.value.replace(/\D/g, ""))}
                  />
                </label>
              </div>
            )}
            {shown.length === 0 ? (
              <p className="text-sm text-muted-foreground">{t("reference.nothing_found")}</p>
            ) : (
              <ul className="min-h-0 flex-1 divide-y overflow-y-auto border">
                {shown.map((e) => (
                  <li key={e.id}>
                    <button
                      type="button"
                      onClick={() => setReferenceId(e.id)}
                      className={cn(
                        "flex w-full items-baseline gap-2 px-3 py-1.5 text-left text-sm hover:bg-accent",
                        e.id === current?.id && "bg-accent text-accent-foreground",
                      )}
                    >
                      <span className="min-w-0 flex-1">
                        <span className="block truncate font-medium">{e.name}</span>
                        {e.subtitle && (
                          <span className="block truncate text-xs text-muted-foreground">{e.subtitle}</span>
                        )}
                      </span>
                      <span className="num shrink-0 text-muted-foreground">{e.value}</span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
          <div className="min-h-0 overflow-y-auto border p-4">
            {current ? (
              <ReferenceDetail entry={current} />
            ) : (
              <p className="text-sm text-muted-foreground">{t("reference.pick")}</p>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
