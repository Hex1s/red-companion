// Chromatic aberration for a whole block (text, board, portrait): red and blue channels shifted apart.
// `drop-shadow` cannot do it: a shadow hides behind opaque content and shows only along the edges.
// Used by `glitch-rgb` in index.css; each variant is one frame of the glitch. Mount once, in the shell.
const SHIFTS = [
  { id: "glitch-a", r: -5, b: 4 },
  { id: "glitch-b", r: 4, b: -3 },
  { id: "glitch-c", r: -3, b: 5 },
  { id: "glitch-d", r: 5, b: -4 },
] as const;

export function GlitchFilters() {
  return (
    <svg aria-hidden width="0" height="0" className="pointer-events-none absolute">
      <defs>
        {SHIFTS.map(({ id, r, b }) => (
          <filter key={id} id={id} x="-5%" y="-5%" width="110%" height="110%" colorInterpolationFilters="sRGB">
            <feColorMatrix in="SourceGraphic" type="matrix" values="1 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 1 0" result="red" />
            <feColorMatrix in="SourceGraphic" type="matrix" values="0 0 0 0 0  0 1 0 0 0  0 0 0 0 0  0 0 0 1 0" result="green" />
            <feColorMatrix in="SourceGraphic" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 1 0 0  0 0 0 1 0" result="blue" />
            <feOffset in="red" dx={r} dy="0" result="redShifted" />
            <feOffset in="blue" dx={b} dy="0" result="blueShifted" />
            <feBlend in="redShifted" in2="green" mode="screen" result="rg" />
            <feBlend in="rg" in2="blueShifted" mode="screen" />
          </filter>
        ))}
      </defs>
    </svg>
  );
}
