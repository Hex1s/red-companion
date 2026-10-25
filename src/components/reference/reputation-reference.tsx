import { useTranslation } from "react-i18next";
import { REPUTATION_RULES } from "@/lib/reference-entries";
import { cn } from "@/lib/utils";

const LEVELS = Array.from({ length: 10 }, (_, i) => i + 1);

type ReputationReferenceProps = {
  /** Entry id picked in the global search (`rule_<key>` / `level_<n>`); that row is marked. */
  currentId: string | null;
};

/** Reputation has no catalog: three rules and ten levels fit on one page, no clicking through. */
export function ReputationReference({ currentId }: ReputationReferenceProps) {
  const { t } = useTranslation();
  return (
    <div className="min-h-0 flex-1 space-y-4 overflow-y-auto">
      <div className="grid gap-3 lg:grid-cols-3">
        {REPUTATION_RULES.map((key) => (
          <section
            key={key}
            className={cn("cut space-y-1 p-3 [--cut:6px]", currentId === `rule_${key}` && "[--cut-border:var(--primary)]")}
          >
            <h3 className="text-sm">{t(`reputation.rule.${key}.name`)}</h3>
            <p className="text-sm whitespace-pre-line text-muted-foreground">
              {t(`reputation.rule.${key}.detail`)}
            </p>
          </section>
        ))}
      </div>

      <ol className="divide-y border">
        {LEVELS.map((level) => (
          <li
            key={level}
            className={cn("flex items-baseline gap-4 px-3 py-2", currentId === `level_${level}` && "bg-accent")}
          >
            <span className="num w-8 shrink-0 text-right font-display text-xl font-bold text-emphasis">{level}</span>
            <span className="text-sm">{t(`reputation.levels.${level}`)}</span>
          </li>
        ))}
      </ol>
    </div>
  );
}
