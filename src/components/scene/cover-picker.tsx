import { useState } from "react";
import { Shield } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { CoverObject } from "@/bindings/CoverObject";
import type { CoverPreset } from "@/bindings/CoverPreset";
import type { Scene } from "@/bindings/Scene";
import { Button } from "@/components/ui/button";
import { FieldBadge } from "@/components/ui/field-badge";
import { Hint } from "@/components/ui/hint";
import { Input } from "@/components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { coverLabel, presetLabel } from "@/lib/cover-label";
import { cn } from "@/lib/utils";

type CoverPickerProps = {
  scene: Scene;
  presets: CoverPreset[];
  /** Cover the character stands behind now. */
  current: CoverObject | null;
  /** Puts the character behind an existing cover, or takes them out (`null`). */
  onAssign: (coverId: number | null) => void;
  /** Creates a cover (a preset, or a hand-made one with `preset = null`) and puts the character behind it. */
  onCreate: (preset: string | null, name: string, hp: number | null) => void;
  /** Sets the cover's current HP. */
  onSetHp: (cover: CoverObject, hp: number) => void;
};

/** Whole number >= 0 from an input; `null` for empty or invalid text. */
function parseHp(text: string): number | null {
  const n = Number(text);
  return text.trim() !== "" && Number.isInteger(n) && n >= 0 ? n : null;
}

/**
 * Cover button of a card: shows the current cover HP and opens the list of what the GM can pick,
 * the covers already on the scene first, then the presets, then a hand-made one.
 */
export function CoverPicker({ scene, presets, current, onAssign, onCreate, onSetHp }: CoverPickerProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [hpText, setHpText] = useState("");
  const [presetId, setPresetId] = useState<string | null>(null);
  const [hpEdit, setHpEdit] = useState("");

  const destroyed = current !== null && current.hp === 0;
  const others = scene.covers.filter((c) => c.id !== current?.id);
  const customHp = parseHp(hpText);

  const close = () => {
    setOpen(false);
    setName("");
    setHpText("");
    setPresetId(null);
  };

  const pickPreset = (preset: CoverPreset) => {
    if (preset.hp === null) {
      // Corpses and living shields have no fixed HP: the GM types it into the form below.
      setPresetId(preset.id);
      setName(presetLabel(preset, t));
      setHpText("");
      return;
    }
    onCreate(preset.id, "", null);
    close();
  };

  const createCustom = () => {
    if (customHp === null || customHp < 1) return;
    onCreate(presetId, presetId ? "" : name, customHp);
    close();
  };

  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next);
        setHpEdit(current ? String(current.hp) : "");
      }}
    >
      <Hint label={current ? `${coverLabel(current, presets, t)} · ${current.hp}/${current.max_hp}` : t("card.cover")}>
        <PopoverTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            size="xs"
            aria-label={t("card.cover")}
            className={cn(current && (destroyed ? "text-state-dead" : "text-cover"))}
          >
            <Shield />
            {current && <span className="num">{current.hp}</span>}
          </Button>
        </PopoverTrigger>
      </Hint>
      <PopoverContent align="end" className="w-80 space-y-3">
        {current && (
          <section className="space-y-2">
            <div className="flex items-baseline justify-between gap-2">
              <span className="font-display text-sm font-semibold uppercase">
                {coverLabel(current, presets, t)}
              </span>
              <span className="num text-xs text-muted-foreground">
                {t("scene.max_hp")} {current.max_hp}
              </span>
            </div>
            <div className="flex items-center gap-2">
              <Input
                inputMode="numeric"
                value={hpEdit}
                aria-label={t("scene.hp")}
                onChange={(e) => setHpEdit(e.target.value)}
                onBlur={() => {
                  const hp = parseHp(hpEdit);
                  if (hp === null || hp > current.max_hp) setHpEdit(String(current.hp));
                  else if (hp !== current.hp) onSetHp(current, hp);
                }}
                onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLElement).blur()}
                className="num h-8 w-20"
              />
              <span className="text-xs text-muted-foreground">{t("scene.hp")}</span>
              <Button
                type="button"
                size="xs"
                variant="outline"
                className="ml-auto"
                onClick={() => {
                  onAssign(null);
                  close();
                }}
              >
                {t("scene.leave")}
              </Button>
            </div>
          </section>
        )}

        {others.length > 0 && (
          <section className="space-y-1">
            <h4 className="text-xs text-muted-foreground">{t("card.cover_on_scene")}</h4>
            <ul className="max-h-32 space-y-1 overflow-y-auto">
              {others.map((cover) => (
                <li key={cover.id}>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="w-full justify-between"
                    onClick={() => {
                      onAssign(cover.id);
                      close();
                    }}
                  >
                    <span className="truncate">{coverLabel(cover, presets, t)}</span>
                    <span className={cn("num", cover.hp === 0 && "text-state-dead")}>
                      {cover.hp}/{cover.max_hp}
                      {cover.characters.length > 0 && ` · ×${cover.characters.length}`}
                    </span>
                  </Button>
                </li>
              ))}
            </ul>
          </section>
        )}

        <section className="space-y-1">
          <h4 className="text-xs text-muted-foreground">{t("scene.preset")}</h4>
          <ul className="max-h-40 space-y-1 overflow-y-auto">
            {presets.map((preset) => (
              <li key={preset.id}>
                <Button
                  type="button"
                  variant={presetId === preset.id ? "default" : "outline"}
                  size="sm"
                  className="w-full justify-between"
                  onClick={() => pickPreset(preset)}
                >
                  <span className="truncate">{presetLabel(preset, t)}</span>
                  <span className="num">{preset.hp ?? t("card.cover_hp_ask")}</span>
                </Button>
              </li>
            ))}
          </ul>
        </section>

        <section className="space-y-1">
          <h4 className="text-xs text-muted-foreground">{t("scene.custom")}</h4>
          <Input
            value={name}
            placeholder={t("scene.name_optional")}
            aria-label={t("scene.name")}
            onChange={(e) => setName(e.target.value)}
            className="h-8"
          />
          <div className="flex items-center gap-2">
            <FieldBadge
              error={hpText.trim() !== "" && (customHp === null || customHp < 1) ? t("validation.cover_hp") : null}
            >
              {(invalid) => (
                <Input
                  inputMode="numeric"
                  value={hpText}
                  placeholder={t("scene.hp")}
                  aria-label={t("scene.hp")}
                  aria-invalid={invalid}
                  onChange={(e) => setHpText(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && createCustom()}
                  className="num h-8 w-20"
                />
              )}
            </FieldBadge>
            <Button
              type="button"
              size="sm"
              className="flex-1"
              disabled={customHp === null || customHp < 1}
              onClick={createCustom}
            >
              {t("scene.add")}
            </Button>
          </div>
        </section>
      </PopoverContent>
    </Popover>
  );
}
