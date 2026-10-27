import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { ErrorNotice } from "@/components/ui/error-notice";
import { api, hasBackend, type UpdateInfo } from "@/lib/api";
import { errorMessage } from "@/lib/errors";

type AboutDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

type Phase = "idle" | "checking" | "latest" | "available" | "installing" | "handed_over";

type Progress = { downloaded: number; total: number | null };

/** The update flow runs in the Rust backend; the dialog only shows which step it is on. */
export function AboutDialog({ open, onOpenChange }: AboutDialogProps) {
  const { t } = useTranslation();
  const [version, setVersion] = useState<string | null>(null);
  const [phase, setPhase] = useState<Phase>("idle");
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [mobile, setMobile] = useState(false);

  useEffect(() => {
    if (hasBackend) api.isMobile().then(setMobile, () => setMobile(false));
  }, []);

  useEffect(() => {
    if (!open || !hasBackend) return;
    let cancelled = false;
    getVersion().then(
      (v) => {
        if (!cancelled) setVersion(v);
      },
      () => {},
    );
    const unlisten = listen<Progress>("update-progress", (event) => setProgress(event.payload));
    return () => {
      cancelled = true;
      unlisten.then((off) => off());
    };
  }, [open]);

  const check = async () => {
    setError(null);
    setPhase("checking");
    try {
      const found = await api.checkUpdate();
      setUpdate(found);
      setPhase(found ? "available" : "latest");
    } catch (e) {
      setError(errorMessage(e));
      setPhase("idle");
    }
  };

  const install = async () => {
    setError(null);
    setProgress(null);
    setPhase("installing");
    try {
      // Android: the system installer takes over and restarts the app itself after the install.
      if (await api.installUpdate()) await api.restartApp();
      else setPhase("handed_over");
    } catch (e) {
      setError(errorMessage(e));
      setPhase("available");
    }
  };

  const percent = progress?.total ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100)) : null;
  // The window must not close in the middle of an install.
  const handleOpenChange = (next: boolean) => {
    if (phase !== "installing") onOpenChange(next);
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>{t("about.title")}</DialogTitle>
          <DialogDescription>{t("about.description")}</DialogDescription>
        </DialogHeader>
        <div className="space-y-4 text-sm">
          <p className="flex items-center justify-between gap-3">
            <span className="text-muted-foreground">{t("about.version")}</span>
            <span className="num">{version ?? "—"}</span>
          </p>
          <section className="space-y-2 border-t border-border pt-3">
            <h3 className="text-xs text-muted-foreground">{t("about.updates")}</h3>
            {phase === "latest" && <p>{t("about.latest")}</p>}
            {phase === "available" && update && (
              <div className="space-y-2">
                <p>
                  {t("about.available")} <span className="num">{update.version}</span>
                </p>
                {update.notes && (
                  <p className="max-h-40 overflow-y-auto whitespace-pre-wrap text-muted-foreground">{update.notes}</p>
                )}
              </div>
            )}
            {phase === "handed_over" && <p>{t("about.handed_over")}</p>}
            {phase === "installing" && (
              <p aria-live="polite" className="num">
                {percent === null ? t("about.installing") : t("about.downloading", { percent })}
              </p>
            )}
            <ErrorNotice message={error} />
            <div className="flex gap-2">
              {phase === "available" || phase === "handed_over" ? (
                <Button size="sm" onClick={install}>
                  {t(mobile ? "about.install_mobile" : "about.install")}
                </Button>
              ) : (
                <Button
                  size="sm"
                  variant="outline"
                  disabled={phase === "checking" || phase === "installing"}
                  onClick={check}
                >
                  {phase === "checking" ? t("about.checking") : t("about.check")}
                </Button>
              )}
            </div>
          </section>
        </div>
      </DialogContent>
    </Dialog>
  );
}
