import type { TFunction } from "i18next";
import type { CoverObject } from "@/bindings/CoverObject";
import type { CoverPreset } from "@/bindings/CoverPreset";

/** "Concrete, thick": a preset name with its thickness. */
export function presetLabel(preset: CoverPreset, t: TFunction): string {
  return preset.thickness
    ? `${preset.name}, ${t(`cover.thickness.${preset.thickness}`)}`
    : preset.name;
}

/** Name of a cover on the scene: its own label, the preset it came from, or "own cover". */
export function coverLabel(cover: CoverObject, presets: CoverPreset[], t: TFunction): string {
  if (cover.name) return cover.name;
  const preset = presets.find((p) => p.id === cover.preset);
  return preset ? presetLabel(preset, t) : t("scene.custom");
}
