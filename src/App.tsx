import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { CharacterKind } from "@/bindings/CharacterKind";
import { CharacterCard } from "@/components/character/character-card";
import { CharacterSheet } from "@/components/character/character-sheet";
import { NpcTemplateDialog, type CloneRequest } from "@/components/character/npc-template-dialog";
import { InitiativeTracker } from "@/components/combat/initiative-tracker";
import { StabilizeDialog } from "@/components/combat/stabilize-dialog";
import { DamageDialog } from "@/components/combat/damage-dialog";
import { LastDamage } from "@/components/combat/last-damage";
import { CoverPanel } from "@/components/scene/cover-panel";
import { AppShell } from "@/components/layout/app-shell";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { errorMessage, useCharacters } from "@/hooks/use-characters";
import { useInjuries } from "@/hooks/use-injuries";
import { useNpcTemplates } from "@/hooks/use-npc-templates";
import { useScene } from "@/hooks/use-scene";
import { api, hasBackend } from "@/lib/api";
import { downloadText, pickTextFile } from "@/lib/file-transfer";
import { SUPPORTED_LANGUAGES } from "@/i18n";
import { useUiStore, type AppTab } from "@/store/ui-store";

export default function App() {
  const { t, i18n } = useTranslation();
  const { tab, setTab, selectedCharacterId, selectCharacter, damageTargetId, openDamage } =
    useUiStore();
  const { entries, error, create, save, remove, reload } = useCharacters();
  const sceneState = useScene();
  const injuries = useInjuries();
  const npcTemplates = useNpcTemplates();
  const [templatesOpen, setTemplatesOpen] = useState(false);
  const [stabilizeTargetId, setStabilizeTargetId] = useState<number | null>(null);
  const damageTarget = entries.find((e) => e.id === damageTargetId) ?? null;
  const selected = entries.find((e) => e.id === selectedCharacterId) ?? null;

  const combat = sceneState.scene?.combat ?? null;
  // Fighters first, in acting order; everyone else keeps the database order.
  const railEntries = combat
    ? [
        ...combat.combatants.flatMap((c) => entries.filter((e) => e.id === c.character_id)),
        ...entries.filter((e) => !combat.combatants.some((c) => c.character_id === e.id)),
      ]
    : entries;

  const createCharacter = async (kind: CharacterKind) => {
    const record = await create(kind, t(`characters.default_name_${kind}`));
    selectCharacter(record.id);
  };

  /** Clones a ready-made NPC `names.length` times; optionally rolls each into the initiative order. */
  const cloneTemplate = async ({ templateId, names, notes, intoCombat }: CloneRequest) => {
    let last: number | null = null;
    try {
      for (const name of names) {
        const record = await api.createNpcFromTemplate(templateId, name, notes);
        last = record.id;
        if (intoCombat) await sceneState.combatAdd(record.id, null);
      }
    } finally {
      // Even after a partial failure the clones that exist must show up.
      await reload();
    }
    if (last !== null) selectCharacter(last);
    if (intoCombat) setTab("combat");
  };

  const [transferNote, setTransferNote] = useState<string | null>(null);

  const exportCharacters = async (ids: number[] | null) => {
    try {
      const name = ids ? (selected?.character.name ?? "character") : "characters";
      downloadText(`${name.replace(/[\\/:*?"<>|]/g, "_")}.redcompanion.json`, await api.exportCharacters(ids));
      setTransferNote(null);
    } catch (e) {
      setTransferNote(t("characters.export_error", { error: errorMessage(e) }));
    }
  };

  const importCharacters = async () => {
    try {
      const json = await pickTextFile(".json,application/json");
      if (json === null) return;
      const imported = await api.importCharacters(json);
      await reload();
      if (imported.length === 1) selectCharacter(imported[0].id);
      setTransferNote(t("characters.imported", { count: imported.length }));
    } catch (e) {
      setTransferNote(t("characters.import_error", { error: errorMessage(e) }));
    }
  };

  const undoDamage = async () => {
    const result = await api.undoDamage();
    sceneState.replaceScene(result.scene);
    await reload();
  };

  return (
    <AppShell
      header={
        <>
          <h1 className="text-lg font-bold text-primary">{t("app.title")}</h1>
          <Tabs value={tab} onValueChange={(v) => setTab(v as AppTab)} className="flex-1">
            <TabsList>
              <TabsTrigger value="combat">{t("nav.combat")}</TabsTrigger>
              <TabsTrigger value="characters">{t("nav.characters")}</TabsTrigger>
              <TabsTrigger value="rules">{t("nav.rules")}</TabsTrigger>
            </TabsList>
          </Tabs>
          <div className="flex gap-1">
            {SUPPORTED_LANGUAGES.map((lng) => (
              <Button
                key={lng}
                size="xs"
                variant={i18n.resolvedLanguage === lng ? "default" : "outline"}
                onClick={() => i18n.changeLanguage(lng)}
              >
                {lng}
              </Button>
            ))}
          </div>
        </>
      }
      main={
        tab === "characters" ? (
          <div className="space-y-6">
            <div className="flex items-center gap-2">
              <h2 className="mr-auto text-2xl font-bold">{t("characters.title")}</h2>
              <Button disabled={!hasBackend} onClick={() => createCharacter("pc")}>
                {t("characters.new_pc")}
              </Button>
              <Button variant="outline" disabled={!hasBackend} onClick={() => createCharacter("npc")}>
                {t("characters.new_npc")}
              </Button>
              <Button
                variant="outline"
                disabled={!hasBackend || npcTemplates.length === 0}
                onClick={() => setTemplatesOpen(true)}
              >
                {t("npc.from_template")}
              </Button>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <Button variant="outline" size="xs" disabled={!hasBackend} onClick={importCharacters}>
                {t("characters.import")}
              </Button>
              <Button
                variant="outline"
                size="xs"
                disabled={!hasBackend || !selected}
                onClick={() => exportCharacters(selected ? [selected.id] : null)}
              >
                {t("characters.export_selected")}
              </Button>
              <Button
                variant="outline"
                size="xs"
                disabled={!hasBackend || entries.length === 0}
                onClick={() => exportCharacters(null)}
              >
                {t("characters.export_all")}
              </Button>
              {transferNote && <span className="text-sm text-muted-foreground">{transferNote}</span>}
            </div>
            {!hasBackend && <p className="text-muted-foreground">{t("characters.no_backend")}</p>}
            {error && (
              <p className="text-destructive">{t("characters.load_error", { error })}</p>
            )}
            {selected ? (
              <CharacterSheet
                key={selected.id}
                entry={selected}
                onSave={(character) => save(selected.id, character)}
                onDelete={async () => {
                  try {
                    await remove(selected.id);
                    selectCharacter(null);
                  } catch (e) {
                    window.alert(errorMessage(e));
                  }
                }}
              />
            ) : (
              hasBackend && (
                <p className="text-muted-foreground">
                  {entries.length === 0 ? t("characters.empty") : t("characters.select_hint")}
                </p>
              )
            )}
          </div>
        ) : tab === "combat" ? (
          <div className="space-y-6">
            <h2 className="text-2xl font-bold">{t("scene.title")}</h2>
            {!hasBackend && <p className="text-muted-foreground">{t("scene.no_backend")}</p>}
            {sceneState.error && (
              <p className="text-destructive">
                {t("scene.load_error", { error: sceneState.error })}
              </p>
            )}
            {sceneState.scene?.last_damage && (
              <LastDamage
                event={sceneState.scene.last_damage}
                onUndo={() => undoDamage().catch((e: unknown) => window.alert(errorMessage(e)))}
              />
            )}
            {combat && (
              <InitiativeTracker
                combat={combat}
                characters={entries.map((e) => ({
                  id: e.id,
                  name: e.character.name,
                  wound: e.derived.wound_state,
                  body: e.character.stats.body,
                  deathSavePenalty: e.character.death_save_penalty,
                }))}
                onAdd={sceneState.combatAdd}
                onReroll={sceneState.combatReroll}
                onSetInitiative={sceneState.combatSetInitiative}
                onShift={sceneState.combatShift}
                onRemove={sceneState.combatRemove}
                onStart={sceneState.combatStart}
                onNextTurn={sceneState.combatNextTurn}
                onEnd={sceneState.combatEnd}
                onDeathSave={async (id, manual) => {
                  try {
                    const applied = await api.deathSave(id, manual);
                    sceneState.replaceScene(applied.scene);
                    await reload();
                    return applied.save;
                  } catch (e) {
                    window.alert(errorMessage(e));
                    return null;
                  }
                }}
                onStabilize={setStabilizeTargetId}
              />
            )}
            {sceneState.scene && (
              <CoverPanel
                scene={sceneState.scene}
                presets={sceneState.presets}
                characters={entries.map((e) => ({ id: e.id, name: e.character.name }))}
                onAdd={sceneState.addCover}
                onEdit={sceneState.editCover}
                onRemove={sceneState.removeCover}
                onSetCover={sceneState.setCharacterCover}
              />
            )}
          </div>
        ) : (
          <p className="text-muted-foreground">{t("app.subtitle")}</p>
        )
      }
      rail={
        <>
          {railEntries.map((e) => (
            <CharacterCard
              key={e.id}
              active={combat?.active === e.id}
              character={{
                id: String(e.id),
                name: e.character.name,
                initiative: combat?.combatants.find((c) => c.character_id === e.id)?.initiative,
                hp: e.character.hp,
                maxHp: e.derived.max_hp,
                spHead: e.character.armor_head.sp,
                spBody: e.character.armor_body.sp,
              }}
              selected={e.id === selectedCharacterId}
              onSelect={() => {
                selectCharacter(e.id);
                setTab("characters");
              }}
              onDamage={() => openDamage(e.id)}
            />
          ))}
          <NpcTemplateDialog
            open={templatesOpen}
            templates={npcTemplates}
            existingNames={entries.map((e) => e.character.name)}
            onClone={cloneTemplate}
            onClose={() => setTemplatesOpen(false)}
          />
          <StabilizeDialog
            target={entries.find((e) => e.id === stabilizeTargetId) ?? null}
            helpers={entries}
            onDone={() => void reload()}
            onClose={() => setStabilizeTargetId(null)}
          />
          {sceneState.scene && (
            <DamageDialog
              target={damageTarget}
              scene={sceneState.scene}
              injuries={injuries}
              onApplied={(result) => {
                sceneState.replaceScene(result.scene);
                void reload();
              }}
              onUndo={undoDamage}
              onClose={() => openDamage(null)}
            />
          )}
        </>
      }
    />
  );
}
