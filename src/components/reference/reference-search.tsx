import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ReferenceDetail } from "@/components/reference/reference-detail";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useReferenceData } from "@/hooks/use-reference-data";
import { allEntries, fold, REFERENCE_SECTIONS, type RefEntry } from "@/lib/reference-entries";
import { cn } from "@/lib/utils";
import { useUiStore } from "@/store/ui-store";

/** Results shown per section, so one big section does not push the others out. */
const PER_SECTION = 8;

type ReferenceSearchProps = { open: boolean; onOpenChange: (open: boolean) => void };

/**
 * Quick lookup over every reference section, as an overlay above any page: the sheet underneath
 * stays where it is. Arrow keys move through the results, Enter jumps to the section page.
 */
export function ReferenceSearch({ open, onOpenChange }: ReferenceSearchProps) {
  const { t } = useTranslation();
  const openReference = useUiStore((s) => s.openReference);
  const data = useReferenceData(open);
  const [query, setQuery] = useState("");
  const [index, setIndex] = useState(0);
  const listRef = useRef<HTMLUListElement>(null);

  const entries = useMemo(() => (open ? allEntries(data, t) : []), [open, data, t]);
  const results = useMemo(() => {
    const q = fold(query.trim());
    if (!q) return [];
    return REFERENCE_SECTIONS.flatMap((section) =>
      entries.filter((e) => e.section === section && e.search.includes(q)).slice(0, PER_SECTION),
    );
  }, [entries, query]);

  const current: RefEntry | null = results[index] ?? null;

  useEffect(() => {
    if (!open) {
      setQuery("");
      setIndex(0);
    }
  }, [open]);

  useEffect(() => {
    listRef.current?.querySelector("[data-current=true]")?.scrollIntoView({ block: "nearest" });
  }, [index, results]);

  const jump = (entry: RefEntry) => {
    openReference(entry.section, entry.id);
    onOpenChange(false);
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      if (results.length === 0) return;
      const step = e.key === "ArrowDown" ? 1 : -1;
      setIndex((i) => (i + step + results.length) % results.length);
    } else if (e.key === "Enter" && current) {
      e.preventDefault();
      jump(current);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-4xl" onKeyDown={onKeyDown}>
        <DialogTitle className="sr-only">{t("reference.global_title")}</DialogTitle>
        <DialogDescription className="sr-only">{t("reference.global_hint")}</DialogDescription>
        <Input
          autoFocus
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setIndex(0);
          }}
          placeholder={t("reference.global_placeholder")}
          aria-label={t("reference.global_title")}
        />
        <div className="grid h-[28rem] grid-cols-[minmax(0,16rem)_minmax(0,1fr)] gap-4">
          <ul ref={listRef} className="min-h-0 overflow-y-auto border">
            {query.trim() === "" ? (
              <li className="p-3 text-sm text-muted-foreground">{t("reference.global_hint")}</li>
            ) : results.length === 0 ? (
              <li className="p-3 text-sm text-muted-foreground">{t("reference.nothing_found")}</li>
            ) : (
              results.map((e, i) => (
                <li key={`${e.section}:${e.id}`}>
                  {(i === 0 || results[i - 1].section !== e.section) && (
                    <div className="sticky top-0 bg-background px-3 py-1 text-xs text-muted-foreground uppercase">
                      {t(`reference.section.${e.section}`)}
                    </div>
                  )}
                  <button
                    type="button"
                    data-current={i === index}
                    onClick={() => setIndex(i)}
                    onDoubleClick={() => jump(e)}
                    className={cn(
                      "flex w-full items-baseline gap-2 px-3 py-1.5 text-left text-sm hover:bg-accent",
                      i === index && "bg-accent text-accent-foreground",
                    )}
                  >
                    <span className="min-w-0 flex-1 truncate">{e.name}</span>
                    <span className="num shrink-0 text-muted-foreground">{e.value}</span>
                  </button>
                </li>
              ))
            )}
          </ul>
          <div className="min-h-0 overflow-y-auto border p-4">
            {current ? (
              <ReferenceDetail entry={current} data={data} />
            ) : (
              <p className="text-sm text-muted-foreground">{t("reference.pick")}</p>
            )}
          </div>
        </div>
        <div className="flex items-center justify-between gap-2 text-xs text-muted-foreground">
          <span>{t("reference.global_keys")}</span>
          <Button size="xs" variant="outline" disabled={!current} onClick={() => current && jump(current)}>
            {t("reference.open_in_section")}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
