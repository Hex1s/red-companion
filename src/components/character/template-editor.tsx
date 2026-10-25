import { useEffect, useState } from "react";
import type { Character } from "@/bindings/Character";
import type { CriticalInjury } from "@/bindings/CriticalInjury";
import type { Derived } from "@/bindings/Derived";
import type { NpcTemplate } from "@/bindings/NpcTemplate";
import type { UserTemplateRecord } from "@/bindings/UserTemplateRecord";
import { CharacterSheet } from "@/components/character/character-sheet";
import { api } from "@/lib/api";

const nothing = () => undefined;

type TemplateEditorProps = {
  /** The own template being edited, if there is one (an own template, or the GM's copy of a book one). */
  record: UserTemplateRecord | null;
  /** The book template being opened; without a `record` the editor starts from its printed sheet. */
  book: NpcTemplate | null;
  /** Weapons and gear text for the sheet of a book template that has no copy yet. */
  notes: string;
  injuries: CriticalInjury[];
  onUpdate: (id: number, character: Character) => Promise<unknown>;
  /** The first edit of a book template: the edited sheet becomes the GM's copy of it. */
  onFork: (code: string, character: Character) => Promise<unknown>;
};

/**
 * A template opened like a character: the same sheet, saved back into the template. A book template
 * is shown as printed until the first edit; then the edited sheet is stored as the GM's own copy
 * (the original stays untouched, deleting the copy brings it back). The parent keys this component
 * by the book code, so the switch from "printed" to "copy" does not remount the sheet mid-typing.
 *
 * The template is a fresh sheet, so the actions that need a live character (healing, injuries, doses,
 * humanity loss) do nothing here.
 */
export function TemplateEditor({ record, book, notes, injuries, onUpdate, onFork }: TemplateEditorProps) {
  const [printed, setPrinted] = useState<Character | null>(null);
  const [derived, setDerived] = useState<Derived | null>(null);
  const hasRecord = record !== null;
  const bookCode = book?.code ?? null;

  useEffect(() => {
    if (hasRecord || bookCode === null) return;
    api.bookTemplateSheet(bookCode, notes).then(setPrinted, nothing);
    // The notes follow the language; the printed sheet is only needed until the first edit.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hasRecord, bookCode]);

  const character = record?.character ?? printed;

  useEffect(() => {
    if (character) api.previewDerived(character).then(setDerived, nothing);
  }, [character]);

  if (!character || !derived) return null;
  return (
    <CharacterSheet
      entry={{ id: record?.id ?? 0, character, derived, kind: "npc" }}
      asTemplate
      onSave={(edited) => (record ? onUpdate(record.id, edited) : onFork(bookCode ?? "", edited))}
      onSaveAsTemplate={async () => undefined}
      injuryTable={injuries}
      onHeal={nothing}
      onAddInjury={nothing}
      onRemoveInjury={nothing}
      onLoseHumanity={nothing}
      onTherapy={nothing}
      onAdminister={nothing}
    />
  );
}
