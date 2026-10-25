/** Drag payload type: the id of a character dragged from the explorer onto a session, a folder or the fight. */
export const CHARACTER_DRAG_TYPE = "application/x-red-companion-character";

/** The character id carried by a drag event, or null when it carries something else. */
export function draggedCharacter(e: { dataTransfer: DataTransfer }): number | null {
  if (!e.dataTransfer.types.includes(CHARACTER_DRAG_TYPE)) return null;
  const id = Number(e.dataTransfer.getData(CHARACTER_DRAG_TYPE));
  return Number.isNaN(id) ? null : id;
}

/** Drag payload type: the id of a folder dragged onto another folder or onto the library. */
export const FOLDER_DRAG_TYPE = "application/x-red-companion-folder";

/** The folder id carried by a drag event, or null when it carries something else. */
export function draggedFolder(e: { dataTransfer: DataTransfer }): number | null {
  if (!e.dataTransfer.types.includes(FOLDER_DRAG_TYPE)) return null;
  const id = Number(e.dataTransfer.getData(FOLDER_DRAG_TYPE));
  return Number.isNaN(id) ? null : id;
}
