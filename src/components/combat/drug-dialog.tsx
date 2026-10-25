import { useState } from "react";
import { Syringe } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { DrugApplied } from "@/bindings/DrugApplied";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { errorMessage } from "@/lib/errors";
import type { CharacterEntry } from "@/hooks/use-characters";
import { useRoles } from "@/hooks/use-roles";
import { api } from "@/lib/api";

export type DrugRequest = {
  /** The Medtech giving the drug. */
  helperId: number;
  /** Drug code, an option of Medtech Pharmaceutics. */
  drug: string;
};

type DrugDialogProps = {
  request: DrugRequest | null;
  /** Everyone who can be given the dose: the session, and the Medtech themselves. */
  targets: CharacterEntry[];
  onDone: (applied: DrugApplied, drugName: string) => void;
  onClose: () => void;
};

/** "Give a drug" (p. 150): pick who gets it; the rules apply the effect and the once-a-period limit. */
export function DrugDialog({ request, targets, onDone, onClose }: DrugDialogProps) {
  return (
    <Dialog open={request !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-sm" aria-describedby={undefined}>
        {request && (
          <DrugForm
            key={`${request.helperId}:${request.drug}`}
            request={request}
            targets={targets}
            onDone={onDone}
            onClose={onClose}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

type DrugFormProps = Omit<DrugDialogProps, "request"> & { request: DrugRequest };

function DrugForm({ request, targets, onDone, onClose }: DrugFormProps) {
  const { t } = useTranslation();
  const roles = useRoles();
  const [targetId, setTargetId] = useState(() => String(request.helperId));
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const option = roles
    .find((r) => r.code === "medtech")
    ?.subabilities.flatMap((s) => s.options)
    .find((o) => o.code === request.drug);
  const helper = targets.find((e) => e.id === request.helperId);

  const give = async () => {
    setBusy(true);
    try {
      const applied = await api.administerDrug(Number(targetId), request.helperId, request.drug);
      onDone(applied, option?.name ?? request.drug);
      onClose();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-4">
      <div className="space-y-1">
        <DialogTitle className="text-xl font-bold">{option?.name ?? request.drug}</DialogTitle>
        <DialogDescription>
          {helper && <span className="mr-2 font-semibold text-foreground">{helper.character.name}</span>}
          {option?.summary}
        </DialogDescription>
      </div>
      <Select value={targetId} onValueChange={setTargetId}>
        <SelectTrigger className="w-full" aria-label={t("drug.target")}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {targets.map((e) => (
            <SelectItem key={e.id} value={String(e.id)}>
              {e.character.name}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <p className="text-xs text-muted-foreground">{t("drug.hint")}</p>
      {error && <p className="text-sm text-emphasis">{error}</p>}
      <div className="flex justify-end">
        <Button type="button" disabled={busy} onClick={() => void give()}>
          <Syringe />
          {t("drug.give")}
        </Button>
      </div>
    </div>
  );
}
