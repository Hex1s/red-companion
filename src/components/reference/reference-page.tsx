import { X } from "@/components/ui/icons";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ReferenceDetail } from "@/components/reference/reference-detail";
import { ReferenceTable, type SortState } from "@/components/reference/reference-table";
import { ReputationReference } from "@/components/reference/reputation-reference";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useReferenceData } from "@/hooks/use-reference-data";
import { hasBackend } from "@/lib/api";
import { modalOpen } from "@/lib/hotkeys";
import { columnsOf, sortEntries } from "@/lib/reference-columns";
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
  // Only an entry the filters still show can be open.
  const current = shown.find((e) => e.id === referenceId) ?? null;

  const columns = useMemo(() => columnsOf(referenceSection, t), [referenceSection, t]);
  const [sort, setSort] = useState<SortState>(null);
  // Columns differ per section, so the sort starts over with it.
  useEffect(() => setSort(null), [referenceSection]);
  const sortColumn = sort ? columns.find((c) => c.id === sort.column) : undefined;
  const rows = sort && sortColumn ? sortEntries(shown, sortColumn, sort.dir) : shown;
  // A header cycles ascending → descending → the book's order.
  const cycleSort = (column: string) =>
    setSort((s) => (s?.column !== column ? { column, dir: "asc" } : s.dir === "asc" ? { column, dir: "desc" } : null));

  // Esc closes the card, unless a dialog or popover wants it.
  useEffect(() => {
    if (!current) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !modalOpen()) setReferenceId(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [current, setReferenceId]);

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
      {referenceSection === "reputation" ? (
        <ReputationReference currentId={referenceId} />
      ) : !hasBackend ? (
        <p className="text-muted-foreground">{t("characters.no_backend")}</p>
      ) : (
        <div
          className={cn(
            "relative grid min-h-0 flex-1 gap-4",
            // Narrow: the card covers the list instead of squeezing it.
            current ? "grid-cols-[minmax(0,1fr)] mid:grid-cols-[minmax(0,1fr)_minmax(0,28rem)]" : "grid-cols-[minmax(0,1fr)]",
          )}
        >
          <div className="flex min-h-0 flex-col gap-2">
            <Input
              value={referenceQuery}
              onChange={(e) => setReferenceQuery(e.target.value)}
              placeholder={t("reference.search")}
              aria-label={t("reference.search")}
              className="max-w-md"
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
              <ReferenceTable
                columns={columns}
                entries={rows}
                currentId={current?.id ?? null}
                sort={sort}
                onSort={cycleSort}
                onSelect={(id) => setReferenceId(id === current?.id ? null : id)}
              />
            )}
          </div>
          {current && (
            <aside className="cut flex min-h-0 max-mid:absolute max-mid:inset-0 max-mid:z-10 animate-in flex-col duration-200 fade-in-0 slide-in-from-right-4 [--cut:10px]">
              <div className="flex justify-end p-1">
                <Button
                  type="button"
                  size="icon-sm"
                  variant="ghost"
                  aria-label={t("reference.close")}
                  onClick={() => setReferenceId(null)}
                >
                  <X />
                </Button>
              </div>
              <div className="min-h-0 flex-1 overflow-y-auto px-4 pb-4">
                <ReferenceDetail entry={current} data={data} />
              </div>
            </aside>
          )}
        </div>
      )}
    </div>
  );
}
