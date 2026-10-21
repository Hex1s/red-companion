import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { CoverObject } from "@/bindings/CoverObject";
import type { CoverPreset } from "@/bindings/CoverPreset";
import type { Scene } from "@/bindings/Scene";
import { HpBar } from "@/components/character/hp-bar";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";

const CUSTOM = "custom";
const NOBODY = "nobody";

export type SceneCharacter = { id: number; name: string };

type CoverPanelProps = {
  scene: Scene;
  presets: CoverPreset[];
  characters: SceneCharacter[];
  onAdd: (preset: string | null, name: string, hp: number | null) => void;
  onEdit: (id: number, name: string, hp: number, maxHp: number) => void;
  onRemove: (id: number) => void;
  onSetCover: (characterId: number, coverId: number | null) => void;
};

/** Parses a whole number from an input; `null` for empty or invalid text. */
function parseHp(text: string): number | null {
  const n = Number(text);
  return text.trim() !== "" && Number.isInteger(n) && n >= 0 ? n : null;
}

/** Cover objects of the scene: presets, HP editing, characters behind each cover. */
export function CoverPanel({
  scene,
  presets,
  characters,
  onAdd,
  onEdit,
  onRemove,
  onSetCover,
}: CoverPanelProps) {
  const { t } = useTranslation();

  const presetLabel = (preset: CoverPreset) =>
    preset.thickness
      ? `${t(preset.name_key)}, ${t(`cover.thickness.${preset.thickness}`)}`
      : t(preset.name_key);

  const coverTitle = (cover: CoverObject) => {
    if (cover.name) return cover.name;
    const preset = presets.find((p) => p.id === cover.preset);
    return preset ? presetLabel(preset) : t("scene.custom");
  };

  return (
    <section className="space-y-4">
      <h3 className="text-lg font-bold">{t("scene.covers")}</h3>

      <AddCover presets={presets} presetLabel={presetLabel} onAdd={onAdd} />

      {scene.covers.length === 0 ? (
        <p className="text-muted-foreground">{t("scene.no_covers")}</p>
      ) : (
        <div className="grid gap-3 [grid-template-columns:repeat(auto-fill,minmax(320px,1fr))]">
          {scene.covers.map((cover) => (
            <CoverCard
              key={`${cover.id}:${cover.name}:${cover.hp}:${cover.max_hp}`}
              cover={cover}
              title={coverTitle(cover)}
              characters={characters}
              onEdit={onEdit}
              onRemove={onRemove}
              onSetCover={onSetCover}
            />
          ))}
        </div>
      )}
    </section>
  );
}

type AddCoverProps = {
  presets: CoverPreset[];
  presetLabel: (preset: CoverPreset) => string;
  onAdd: CoverPanelProps["onAdd"];
};

function AddCover({ presets, presetLabel, onAdd }: AddCoverProps) {
  const { t } = useTranslation();
  const [presetId, setPresetId] = useState<string>(CUSTOM);
  const [name, setName] = useState("");
  const [hpText, setHpText] = useState("");

  const preset = presets.find((p) => p.id === presetId) ?? null;
  const hp = parseHp(hpText);
  const needsHp = preset === null || preset.hp === null;
  const invalid = hpText.trim() !== "" ? hp === null || hp < 1 : needsHp;

  const submit = () => {
    if (invalid) return;
    onAdd(preset?.id ?? null, name, hp);
    setName("");
    setHpText("");
  };

  return (
    <div className="cut flex flex-wrap items-end gap-3 p-3 [--cut:8px]">
      <div className="space-y-1">
        <span className="block text-xs text-muted-foreground uppercase">{t("scene.preset")}</span>
        <Select value={presetId} onValueChange={setPresetId}>
          <SelectTrigger className="w-60">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {presets.map((p) => (
              <SelectItem key={p.id} value={p.id}>
                {presetLabel(p)}
                {p.hp !== null && ` · ${p.hp} ${t("scene.hp")}`}
              </SelectItem>
            ))}
            <SelectItem value={CUSTOM}>{t("scene.custom")}</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div className="space-y-1">
        <span className="block text-xs text-muted-foreground uppercase">{t("scene.name_optional")}</span>
        <Input value={name} onChange={(e) => setName(e.target.value)} className="w-48" />
      </div>
      <div className="space-y-1">
        <span className="block text-xs text-muted-foreground uppercase">{t("scene.hp")}</span>
        <Input
          inputMode="numeric"
          value={hpText}
          placeholder={preset?.hp != null ? String(preset.hp) : t("scene.hp_required")}
          onChange={(e) => setHpText(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && submit()}
          aria-invalid={hpText.trim() !== "" && invalid}
          className="num w-28"
        />
      </div>
      <Button disabled={invalid} onClick={submit}>
        {t("scene.add")}
      </Button>
    </div>
  );
}

type CoverCardProps = {
  cover: CoverObject;
  title: string;
  characters: SceneCharacter[];
  onEdit: CoverPanelProps["onEdit"];
  onRemove: CoverPanelProps["onRemove"];
  onSetCover: CoverPanelProps["onSetCover"];
};

function CoverCard({ cover, title, characters, onEdit, onRemove, onSetCover }: CoverCardProps) {
  const { t } = useTranslation();
  const [name, setName] = useState(cover.name);
  const [hpText, setHpText] = useState(String(cover.hp));
  const [maxText, setMaxText] = useState(String(cover.max_hp));
  const destroyed = cover.hp === 0;

  // Fields commit on blur / Enter; invalid text snaps back to the stored values.
  const commit = () => {
    const hp = parseHp(hpText);
    const max = parseHp(maxText);
    if (hp === null || max === null || max < 1) {
      setName(cover.name);
      setHpText(String(cover.hp));
      setMaxText(String(cover.max_hp));
      return;
    }
    if (name.trim() !== cover.name || hp !== cover.hp || max !== cover.max_hp) {
      onEdit(cover.id, name, hp, max);
    }
  };
  const onEnter = (e: React.KeyboardEvent) => e.key === "Enter" && (e.target as HTMLElement).blur();

  const behind = cover.characters
    .map((id) => characters.find((c) => c.id === id))
    .filter((c): c is SceneCharacter => c !== undefined);
  const available = characters.filter((c) => !cover.characters.includes(c.id));

  return (
    <div
      className={cn(
        "cut space-y-3 p-3 [--cut:8px]",
        destroyed && "[--cut-border:var(--state-dead)]",
      )}
    >
      <div className="flex items-baseline justify-between gap-2">
        <h4 className={cn("truncate text-sm font-semibold", destroyed && "text-muted-foreground")}>
          {title}
        </h4>
        {destroyed && (
          <span className="text-xs text-state-dead uppercase">{t("scene.destroyed")}</span>
        )}
      </div>

      <HpBar current={cover.hp} max={cover.max_hp} />

      <div className="flex flex-wrap items-end gap-2" onBlur={commit}>
        <label className="block space-y-1">
          <span className="block text-xs text-muted-foreground uppercase">{t("scene.name")}</span>
          <Input
            value={name}
            placeholder={title}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={onEnter}
            className="w-40"
          />
        </label>
        <label className="block space-y-1">
          <span className="block text-xs text-muted-foreground uppercase">{t("scene.hp")}</span>
          <Input
            inputMode="numeric"
            value={hpText}
            onChange={(e) => setHpText(e.target.value)}
            onKeyDown={onEnter}
            className="num w-16"
          />
        </label>
        <label className="block space-y-1">
          <span className="block text-xs text-muted-foreground uppercase">{t("scene.max_hp")}</span>
          <Input
            inputMode="numeric"
            value={maxText}
            onChange={(e) => setMaxText(e.target.value)}
            onKeyDown={onEnter}
            className="num w-16"
          />
        </label>
      </div>

      <div className="space-y-2">
        <span className="block text-xs text-muted-foreground uppercase">{t("scene.behind")}</span>
        {behind.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("scene.nobody")}</p>
        ) : (
          <ul className="space-y-1">
            {behind.map((c) => (
              <li key={c.id} className="flex items-center justify-between gap-2 text-sm">
                <span className="truncate">{c.name}</span>
                <Button size="xs" variant="outline" onClick={() => onSetCover(c.id, null)}>
                  {t("scene.leave")}
                </Button>
              </li>
            ))}
          </ul>
        )}
        {available.length > 0 && (
          <Select value={NOBODY} onValueChange={(id) => onSetCover(Number(id), cover.id)}>
            <SelectTrigger className="w-full">
              <SelectValue>{t("scene.add_character")}</SelectValue>
            </SelectTrigger>
            <SelectContent>
              {available.map((c) => (
                <SelectItem key={c.id} value={String(c.id)}>
                  {c.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
      </div>

      <Button size="xs" variant="outline" onClick={() => onRemove(cover.id)}>
        {t("scene.remove")}
      </Button>
    </div>
  );
}
