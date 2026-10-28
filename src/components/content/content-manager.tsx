import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Loader2 } from "@/components/ui/icons";
import { api, type ContentPack, type ContentStatus } from "@/lib/api";
import { errorMessage } from "@/lib/errors";
import { useContentStore } from "@/store/content-store";

type ContentManagerProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** The first start: the same dialog with an introduction and a way to skip. */
  firstRun?: boolean;
};

const LANGS = ["ru", "en"] as const;

/** Collections of a pack the server has (count above zero). */
const collectionsOf = (pack: ContentPack) => Object.entries(pack.counts).filter(([, n]) => n > 0).map(([name]) => name);

/**
 * Server content: mode (on request / only what was downloaded), languages, and what is downloaded
 * per pack. The mechanics of the rules are built in and are not listed here.
 */
export function ContentManager({ open, onOpenChange, firstRun = false }: ContentManagerProps) {
  const { t, i18n } = useTranslation();
  const bump = useContentStore((s) => s.bump);
  const [state, setState] = useState<ContentStatus | null>(null);
  /** What is running: `check`, `all` or a pack code; the buttons wait for it. */
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try {
      setState(await api.contentStatus());
    } catch (e) {
      setError(errorMessage(e));
    }
  }, []);

  useEffect(() => {
    if (!open) return;
    setError(null);
    setMessage(null);
    void reload();
  }, [open, reload]);

  /** Runs one action: busy while it lasts, its error stays in the dialog. */
  const act = async (key: string, run: () => Promise<string | null | void>) => {
    setBusy(key);
    setError(null);
    setMessage(null);
    try {
      const done = await run();
      if (done) setMessage(done);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(null);
      await reload();
    }
  };

  const changeSettings = (mode: ContentStatus["mode"], langs: string[]) =>
    act("settings", async () => {
      await api.setContentSettings(mode, langs);
    });

  const toggleLang = (code: string, on: boolean) => {
    if (!state) return;
    const langs = on ? [...state.langs, code] : state.langs.filter((l) => l !== code);
    if (langs.length > 0) void changeSettings(state.mode, LANGS.filter((l) => langs.includes(l)));
  };

  const check = () =>
    act("check", async () => {
      const report = await api.syncContent({ langs: [] });
      return t("content.checked", { release: report.release });
    });

  const download = (key: string, packs?: string[]) =>
    act(key, async () => {
      if (!state) return null;
      const report = await api.syncContent({ langs: state.langs, packs: packs ?? null });
      bump();
      return report.downloaded > 0 ? t("content.downloaded", { count: report.downloaded }) : t("content.up_to_date");
    });

  const remove = (pack: ContentPack) =>
    act(pack.code, async () => {
      await api.deleteCachedContent(pack.code);
      bump();
    });

  const togglePack = (pack: ContentPack, enabled: boolean) =>
    act(pack.code, async () => {
      await api.setPackEnabled(pack.code, enabled);
      bump();
    });

  const have = (pack: ContentPack, lang: string) => {
    const names = new Set(collectionsOf(pack));
    return state?.cached.filter((c) => c.pack === pack.code && c.lang === lang && names.has(c.collection)).length ?? 0;
  };

  const working = busy !== null;
  const packName = (pack: ContentPack) => pack.name[i18n.language] ?? pack.name.en ?? pack.code;

  return (
    <Dialog open={open} onOpenChange={(next) => !working && onOpenChange(next)}>
      <DialogContent className="max-w-xl">
        <DialogHeader>
          <DialogTitle>{t("content.title")}</DialogTitle>
          <DialogDescription>{firstRun ? t("content.intro") : t("content.description")}</DialogDescription>
        </DialogHeader>

        {state && (
          <div className="space-y-4 text-sm">
            <section className="space-y-2">
              <h3 className="text-xs text-muted-foreground">{t("content.mode")}</h3>
              <div className="flex flex-wrap gap-2">
                {(["online", "local"] as const).map((mode) => (
                  <Button
                    key={mode}
                    size="sm"
                    variant={state.mode === mode ? "default" : "outline"}
                    disabled={working}
                    onClick={() => void changeSettings(mode, state.langs)}
                  >
                    {t(`content.mode_${mode}`)}
                  </Button>
                ))}
              </div>
              <p className="text-muted-foreground">{t(`content.mode_${state.mode}_hint`)}</p>
            </section>

            <section className="space-y-2">
              <h3 className="text-xs text-muted-foreground">{t("content.languages")}</h3>
              <div className="flex flex-wrap gap-4">
                {LANGS.map((code) => (
                  <label key={code} className="flex items-center gap-2">
                    <Checkbox
                      checked={state.langs.includes(code)}
                      disabled={working || (state.langs.length === 1 && state.langs.includes(code))}
                      onCheckedChange={(on) => toggleLang(code, on === true)}
                    />
                    {t(`content.lang_${code}`)}
                  </label>
                ))}
              </div>
            </section>

            <section className="space-y-2 border-t border-border pt-3">
              <div className="flex flex-wrap items-center gap-2">
                <h3 className="flex-1 text-xs text-muted-foreground">
                  {state.release === null ? t("content.not_checked") : t("content.release", { release: state.release })}
                </h3>
                <Button size="sm" variant="outline" disabled={working} onClick={() => void check()}>
                  {busy === "check" && <Loader2 className="size-4 animate-spin" />}
                  {busy === "check" ? t("content.checking") : t("content.check")}
                </Button>
                <Button size="sm" disabled={working} onClick={() => void download("all")}>
                  {busy === "all" && <Loader2 className="size-4 animate-spin" />}
                  {busy === "all" ? t("content.downloading") : t("content.download_all")}
                </Button>
              </div>
            </section>

            <section className="space-y-2">
              <h3 className="text-xs text-muted-foreground">{t("content.packs")}</h3>
              {state.packs.length === 0 && <p className="text-muted-foreground">{t("content.no_packs")}</p>}
              <ul className="space-y-2">
                {state.packs.map((pack) => (
                  <li key={pack.code} className="space-y-1 border border-border p-2">
                    <div className="flex flex-wrap items-center gap-2">
                      <label className="flex flex-1 items-center gap-2">
                        <Checkbox checked={pack.enabled} disabled={working} onCheckedChange={(on) => void togglePack(pack, on === true)} />
                        <span className="font-display">{packName(pack)}</span>
                      </label>
                      <Button size="xs" variant="outline" disabled={working} onClick={() => void download(pack.code, [pack.code])}>
                        {busy === pack.code && <Loader2 className="size-3.5 animate-spin" />}
                        {t("content.pack_download")}
                      </Button>
                      <Button size="xs" variant="outline" disabled={working} onClick={() => void remove(pack)}>
                        {t("content.pack_remove")}
                      </Button>
                    </div>
                    {pack.summary[i18n.language] && <p className="text-muted-foreground">{pack.summary[i18n.language]}</p>}
                    <p className="num text-xs text-muted-foreground">
                      {state.langs.map((lang) => t("content.pack_have", { lang: lang.toUpperCase(), have: have(pack, lang), total: collectionsOf(pack).length })).join(" · ")}
                    </p>
                  </li>
                ))}
              </ul>
            </section>

            {message && (
              <p role="status" className="num">
                {message}
              </p>
            )}
          </div>
        )}
        <ErrorNotice message={error} />

        {firstRun && (
          <div className="flex justify-end">
            <Button variant="outline" disabled={working} onClick={() => onOpenChange(false)}>
              {t("content.skip")}
            </Button>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
