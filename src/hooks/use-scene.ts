import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { CoverPreset } from "@/bindings/CoverPreset";
import type { Scene } from "@/bindings/Scene";
import type { SceneRecord } from "@/bindings/SceneRecord";
import { api, hasBackend } from "@/lib/api";
import { status } from "@/store/status-store";
import { errorMessage } from "@/lib/errors";

/** Scene (cover objects, fight) of the active session and cover presets from the Rust backend. */
export function useScene(sessionId: number | null) {
  const { i18n } = useTranslation();
  const [scene, setScene] = useState<Scene | null>(null);
  const [presets, setPresets] = useState<CoverPreset[]>([]);
  const [error, setError] = useState<unknown>(null);

  // Every session has its own scene: it is reloaded when the active session changes.
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    if (!hasBackend || sessionId === null) return;
    let cancelled = false;
    setScene(null);
    api.getScene().then(
      (record) => {
        if (cancelled) return;
        setScene(record.scene);
        status.dismiss("scene:load");
      },
      (e: unknown) => {
        if (cancelled) return;
        setError(e ?? "unknown");
        status.fail(e, "scene:load", {
          text: i18n.t("scene.load_error", { error: errorMessage(e) }),
          retry: { label: i18n.t("status.retry"), run: () => setAttempt((n) => n + 1) },
        });
      },
    );
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId, attempt]);

  // Presets are server content: names follow the language.
  useEffect(() => {
    if (!hasBackend) return;
    api.listCoverPresets(i18n.language).then(setPresets, (e: unknown) => setError(e ?? "unknown"));
  }, [i18n.language]);

  /** Runs a scene command; the backend answers with the whole updated scene. */
  const run = useCallback(async (command: Promise<SceneRecord>) => {
    try {
      const record = await command;
      setScene(record.scene);
      setError(null);
      return record;
    } catch (e) {
      setError(e ?? "unknown");
      status.fail(e, "scene");
      return null;
    }
  }, []);

  return {
    scene,
    /** Replaces the scene with a record returned by another command (damage). */
    replaceScene: (record: SceneRecord) => setScene(record.scene),
    presets,
    error,
    addCover: (preset: string | null, name: string, hp: number | null) =>
      run(api.addCover(preset, name, hp)),
    editCover: (id: number, name: string, hp: number, maxHp: number) =>
      run(api.editCover(id, name, hp, maxHp)),
    removeCover: (id: number) => run(api.removeCover(id)),
    setCharacterCover: (characterId: number, coverId: number | null) =>
      run(api.setCharacterCover(characterId, coverId)),
    /** Creates a cover and puts the character behind it. */
    addCoverFor: async (characterId: number, preset: string | null, name: string, hp: number | null) => {
      const created = await run(api.addCover(preset, name, hp));
      // Ids are never reused, so the new cover has the highest one.
      const id = created ? Math.max(...created.scene.covers.map((c) => c.id)) : null;
      if (id !== null) await run(api.setCharacterCover(characterId, id));
    },
    /** Rolls REF + 1d10 for a character (adds them to the order, or re-rolls). */
    combatRoll: (id: number) => run(api.combatRoll(id, null)),
    combatSetInitiative: (id: number, initiative: number) =>
      run(api.combatSetInitiative(id, initiative)),
    combatShift: (id: number, delta: number) => run(api.combatShift(id, delta)),
    combatAdd: (id: number) => run(api.combatAdd(id)),
    combatRemove: (id: number) => run(api.combatRemove(id)),
    combatStart: () => run(api.combatStart()),
    combatNextTurn: () => run(api.combatNextTurn()),
    combatEnd: () => run(api.combatEnd()),
  };
}
