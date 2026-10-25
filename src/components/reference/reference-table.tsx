import { ArrowDown, ArrowUp, ChevronsUpDown } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { RefColumn } from "@/lib/reference-columns";
import type { RefEntry } from "@/lib/reference-entries";
import { cn } from "@/lib/utils";

export type SortState = { column: string; dir: "asc" | "desc" } | null;

type ReferenceTableProps = {
  columns: RefColumn[];
  entries: RefEntry[];
  currentId: string | null;
  sort: SortState;
  /** A header was clicked: the page cycles ascending → descending → unsorted. */
  onSort: (column: string) => void;
  onSelect: (id: string) => void;
};

/** The reference as a table: sortable headers, the chosen row marked, the rest of the page free for the card. */
export function ReferenceTable({ columns, entries, currentId, sort, onSort, onSelect }: ReferenceTableProps) {
  const { t } = useTranslation();
  return (
    <div className="min-h-0 flex-1 overflow-auto border">
      <table className="w-full border-collapse text-sm">
        <thead className="sticky top-0 z-10 bg-card">
          <tr className="border-b">
            {columns.map((c) => {
              const active = sort?.column === c.id;
              return (
                <th
                  key={c.id}
                  scope="col"
                  aria-sort={active ? (sort.dir === "asc" ? "ascending" : "descending") : "none"}
                  className={cn("p-0 font-normal", c.numeric && "text-right")}
                >
                  <button
                    type="button"
                    onClick={() => onSort(c.id)}
                    title={t("reference.sort_by", { column: c.label })}
                    className={cn(
                      "flex w-full items-center gap-1 px-3 py-1.5 text-xs uppercase outline-none hover:bg-accent focus-visible:underline",
                      c.numeric && "justify-end",
                      active ? "text-foreground" : "text-muted-foreground",
                    )}
                  >
                    {c.label}
                    {active ? (
                      sort.dir === "asc" ? <ArrowUp className="size-3" /> : <ArrowDown className="size-3" />
                    ) : (
                      <ChevronsUpDown className="size-3 opacity-40" />
                    )}
                  </button>
                </th>
              );
            })}
          </tr>
        </thead>
        <tbody>
          {entries.map((e) => {
            const selected = e.id === currentId;
            return (
              <tr
                key={e.id}
                tabIndex={0}
                aria-selected={selected}
                onClick={() => onSelect(e.id)}
                onKeyDown={(ev) => {
                  if (ev.key === "Enter" || ev.key === " ") {
                    ev.preventDefault();
                    onSelect(e.id);
                  }
                }}
                className={cn(
                  "cursor-pointer border-b border-l-2 border-l-transparent outline-none last:border-b-0 hover:bg-accent/60 focus-visible:bg-accent",
                  selected && "border-l-primary bg-accent",
                )}
              >
                {columns.map((c, i) => (
                  <td
                    key={c.id}
                    className={cn(
                      "px-3 py-1.5",
                      c.numeric ? "num text-right" : i === 0 ? "font-medium" : "text-muted-foreground",
                    )}
                  >
                    {c.text(e)}
                  </td>
                ))}
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
