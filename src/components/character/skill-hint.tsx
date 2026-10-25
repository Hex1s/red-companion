import type { ReactElement } from "react";
import { useTranslation } from "react-i18next";
import { Hint } from "@/components/ui/hint";

/** The hint texts of a skill (`skill.summary.<id>`, optional `skill.details.<id>`), empty when it has none. */
export function useSkillText(id: string): { summary: string | null; details: string | null } {
  const { t, i18n } = useTranslation();
  const summaryKey = `skill.summary.${id}`;
  const detailsKey = `skill.details.${id}`;
  return {
    summary: i18n.exists(summaryKey) ? t(summaryKey) : null,
    details: i18n.exists(detailsKey) ? t(detailsKey) : null,
  };
}

type SkillHintProps = { id: string; children: ReactElement };

/** Shows what a skill is for when the pointer rests on its name. */
export function SkillHint({ id, children }: SkillHintProps) {
  const { summary, details } = useSkillText(id);
  if (summary === null) return children;
  return (
    <Hint
      label={
        <span className="block max-w-64 space-y-1">
          <span className="block">{summary}</span>
          {details && <span className="block text-muted-foreground">{details}</span>}
        </span>
      }
    >
      {children}
    </Hint>
  );
}
