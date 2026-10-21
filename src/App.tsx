import { useTranslation } from "react-i18next";
import type { CharacterKind } from "@/bindings/CharacterKind";
import { CharacterCard } from "@/components/character/character-card";
import { CharacterSheet } from "@/components/character/character-sheet";
import { CoverPanel } from "@/components/scene/cover-panel";
import { AppShell } from "@/components/layout/app-shell";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { errorMessage, useCharacters } from "@/hooks/use-characters";
import { useScene } from "@/hooks/use-scene";
import { hasBackend } from "@/lib/api";
import { SUPPORTED_LANGUAGES } from "@/i18n";
import { useUiStore, type AppTab } from "@/store/ui-store";

export default function App() {
  const { t, i18n } = useTranslation();
  const { tab, setTab, selectedCharacterId, selectCharacter } = useUiStore();
  const { entries, error, create, save, remove } = useCharacters();
  const sceneState = useScene();
  const selected = entries.find((e) => e.id === selectedCharacterId) ?? null;

  const createCharacter = async (kind: CharacterKind) => {
    const record = await create(kind, t(`characters.default_name_${kind}`));
    selectCharacter(record.id);
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
      rail={entries.map((e) => (
        <CharacterCard
          key={e.id}
          character={{
            id: String(e.id),
            name: e.character.name,
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
        />
      ))}
    />
  );
}
