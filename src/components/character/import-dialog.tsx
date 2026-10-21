import { useState } from "react";
import { FolderOpen } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { CharacterRecord } from "@/bindings/CharacterRecord";
import type { ExportFile } from "@/bindings/ExportFile";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { errorMessage } from "@/hooks/use-characters";
import { api } from "@/lib/api";

type ImportDialogProps = {
  /** Files of the exports folder; the window is open while the list is set. */
  files: ExportFile[] | null;
  onImported: (records: CharacterRecord[]) => void;
  onOpenFolder: () => void;
  onClose: () => void;
};

/** Import: pick one of the files in the app's export folder. Errors stay in the window. */
export function ImportDialog({ files, onImported, onOpenFolder, onClose }: ImportDialogProps) {
  const { t } = useTranslation();
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const pick = async (name: string) => {
    setBusy(name);
    setError(null);
    try {
      onImported(await api.importExportFile(name));
      onClose();
    } catch (e) {
      setError(t("transfer.error", { error: errorMessage(e) }));
    } finally {
      setBusy(null);
    }
  };

  return (
    <Dialog open={files !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-lg" aria-describedby={undefined}>
        <DialogHeader>
          <DialogTitle>{t("transfer.import_title")}</DialogTitle>
          <DialogDescription>{t("transfer.import_hint")}</DialogDescription>
        </DialogHeader>
        <ul className="max-h-[50vh] space-y-1 overflow-y-auto pr-1">
          {(files ?? []).map((file) => (
            <li key={file.name}>
              <Button
                type="button"
                variant="outline"
                className="h-auto w-full justify-between py-2 normal-case"
                disabled={busy !== null}
                onClick={() => void pick(file.name)}
              >
                <span className="truncate">{file.name.replace(/\.redcompanion\.json$/, "")}</span>
                <span className="num text-xs text-muted-foreground">
                  {new Date(file.modified_ms).toLocaleString()}
                </span>
              </Button>
            </li>
          ))}
        </ul>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <div className="flex justify-between gap-2">
          <Button type="button" variant="outline" onClick={onOpenFolder}>
            <FolderOpen />
            {t("transfer.open_folder")}
          </Button>
          <Button type="button" variant="outline" onClick={onClose}>
            {t("characters.cancel")}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
