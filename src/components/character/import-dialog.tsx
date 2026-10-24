import { useState } from "react";
import { FolderOpen } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ExportFile } from "@/bindings/ExportFile";
import { Button } from "@/components/ui/button";
import { ErrorNotice } from "@/components/ui/error-notice";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { errorMessage } from "@/lib/errors";

type ImportDialogProps = {
  /** Files of the exports folder; the window is open while the list is set. */
  files: ExportFile[] | null;
  /** Imports the picked file; a rejection is shown in the window, which stays open. */
  onImport: (name: string) => Promise<void>;
  onOpenFolder: () => void;
  onClose: () => void;
};

/** Import: pick one of the files in the app's export folder. Errors stay in the window. */
export function ImportDialog({ files, onImport, onOpenFolder, onClose }: ImportDialogProps) {
  const { t } = useTranslation();
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const pick = async (name: string) => {
    setBusy(name);
    setError(null);
    try {
      await onImport(name);
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
                <span className="truncate">{file.name.replace(/(\.(character|session|encounters))?\.redcompanion\.json$|\.redencounters\.json$/, "")}</span>
                <span className="num text-xs text-muted-foreground">
                  {new Date(file.modified_ms).toLocaleString()}
                </span>
              </Button>
            </li>
          ))}
        </ul>
        <ErrorNotice message={error} />
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
