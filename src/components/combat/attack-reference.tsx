import { formatHotkey } from "@/lib/hotkeys";
import { BookOpen, Bomb, Crosshair, Rocket, Scan, Sparkles, Target, Wind, Zap } from "lucide-react";
import type { ComponentType } from "react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Article } from "@/bindings/Article";
import type { RangeTable } from "@/bindings/RangeTable";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { api, hasBackend } from "@/lib/api";
import { cn } from "@/lib/utils";
import { useUiStore } from "@/store/ui-store";

/** Icon per weapon class row of the range tables; a class without one falls back to the crosshair. */
const ROW_ICONS: Record<string, ComponentType<{ className?: string }>> = {
  pistol: Crosshair,
  smg: Zap,
  shotgun_slug: Wind,
  assault_rifle: Target,
  sniper_rifle: Scan,
  bow_crossbow: Sparkles,
  grenade_launcher: Bomb,
  rocket_launcher: Rocket,
};

type Selection = { kind: "row"; id: string } | { kind: "article"; id: string } | null;

const bandLabel = (b: { from: number; to: number }) => `${b.from}–${b.to}`;

/** Range tables and attack articles, loaded when the popover first opens. */
function useAttackData(enabled: boolean) {
  const [tables, setTables] = useState<RangeTable[]>([]);
  const [articles, setArticles] = useState<Article[]>([]);
  useEffect(() => {
    if (!enabled || !hasBackend || tables.length > 0) return;
    api.listRangeTables().then(setTables, () => setTables([]));
    api.listArticles().then((all) => setArticles(all.filter((a) => a.tags.includes("attack"))), () => setArticles([]));
  }, [enabled, tables.length]);
  return { tables, articles };
}

/** Quick attack reference: DV by distance for a weapon class, and short cards for fire modes and actions. Reference only. */
export function AttackReference() {
  const { t } = useTranslation();
  const open = useUiStore((s) => s.attackRefOpen);
  const setOpen = useUiStore((s) => s.setAttackRefOpen);
  const [selection, setSelection] = useState<Selection>(null);
  const { tables, articles } = useAttackData(open);

  // Weapon classes in the order of the single-shot table.
  const rows = tables.find((table) => table.id === "single")?.rows ?? [];
  const article = selection?.kind === "article" ? articles.find((a) => a.id === selection.id) : undefined;

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <Hint label={`${t("attackRef.title")} ${formatHotkey("attackRef")}`}>
        <PopoverTrigger asChild>
          <Button type="button" size="icon-sm" variant="outline" aria-label={t("attackRef.title")}>
            <BookOpen />
          </Button>
        </PopoverTrigger>
      </Hint>
      <PopoverContent align="end" className="w-[26rem] space-y-3">
        <div className="font-display text-xs font-semibold uppercase">{t("attackRef.title")}</div>

        <section className="space-y-1.5">
          <div className="text-xs text-muted-foreground uppercase">{t("attackRef.weapons")}</div>
          <div className="flex flex-wrap gap-1.5">
            {rows.map((row) => {
              const Icon = ROW_ICONS[row.id] ?? Crosshair;
              const active = selection?.kind === "row" && selection.id === row.id;
              return (
                <Hint key={row.id} label={t(row.name_key)}>
                  <Button
                    type="button"
                    size="icon-sm"
                    variant={active ? "default" : "outline"}
                    aria-label={t(row.name_key)}
                    aria-pressed={active}
                    onClick={() => setSelection({ kind: "row", id: row.id })}
                  >
                    <Icon />
                  </Button>
                </Hint>
              );
            })}
          </div>
        </section>

        <section className="space-y-1.5">
          <div className="text-xs text-muted-foreground uppercase">{t("attackRef.modes")}</div>
          <div className="flex flex-wrap gap-1.5">
            {articles.map((a) => {
              const active = selection?.kind === "article" && selection.id === a.id;
              return (
                <Button
                  key={a.id}
                  type="button"
                  size="xs"
                  variant={active ? "default" : "outline"}
                  aria-pressed={active}
                  onClick={() => setSelection({ kind: "article", id: a.id })}
                >
                  {t(a.name_key)}
                </Button>
              );
            })}
          </div>
        </section>

        <div className="border-t pt-3">
          {selection?.kind === "row" && <RowTables tables={tables} rowId={selection.id} />}
          {article && (
            <div className="space-y-1.5 text-xs">
              <div className="font-display font-semibold uppercase">{t(article.name_key)}</div>
              <p className="whitespace-pre-line text-muted-foreground">
                {t(article.details_key ?? article.summary_key)}
              </p>
              <div className="num text-muted-foreground">{t("attackRef.page", { page: article.source.page })}</div>
            </div>
          )}
          {!selection && <p className="text-xs text-muted-foreground">{t("attackRef.pickHint")}</p>}
          <p className="mt-2 text-xs text-muted-foreground">{t("attackRef.note")}</p>
        </div>
      </PopoverContent>
    </Popover>
  );
}

type RowTablesProps = { tables: RangeTable[]; rowId: string };

/** DV by distance of one weapon class, from each table that lists it (single shot, autofire). */
function RowTables({ tables, rowId }: RowTablesProps) {
  const { t } = useTranslation();
  const found = tables.flatMap((table) => {
    const row = table.rows.find((r) => r.id === rowId);
    return row ? [{ table, row }] : [];
  });
  if (found.length === 0) return <p className="text-xs text-muted-foreground">{t("attackRef.noRow")}</p>;

  return (
    <div className="space-y-3">
      {found.map(({ table, row }) => (
        <div key={table.id} className="space-y-1">
          <div className="flex items-baseline justify-between text-xs">
            <span className="font-display font-semibold uppercase">
              {t(row.name_key)} · {t(table.name_key)}
            </span>
            <span className="num text-muted-foreground">{t("attackRef.page", { page: table.source.page })}</span>
          </div>
          <div className="grid grid-cols-4 gap-1">
            {table.bands.map((band, i) => {
              const dv = row.dv[i];
              return (
                <div key={bandLabel(band)} className="cut p-1.5 text-center [--cut:4px]">
                  <div className="num text-[10px] text-muted-foreground">{bandLabel(band)}</div>
                  <div className={cn("num text-sm", dv === null ? "text-muted-foreground" : "text-emphasis")}>
                    {dv ?? t("attackRef.impossible")}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      ))}
      <div className="text-[10px] text-muted-foreground uppercase">
        {t("attackRef.distance")} · {t("attackRef.dv")}
      </div>
    </div>
  );
}
