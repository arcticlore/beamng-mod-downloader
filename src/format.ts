/** Человекочитаемое «осталось …» из остатка байт и скорости. */
export function formatEta(
  remaining: number,
  speedBps: number,
  units: { h: string; m: string; s: string },
): string | null {
  if (speedBps <= 0 || remaining <= 0) return null;
  const secs = Math.ceil(remaining / speedBps);
  if (secs >= 3600)
    return `${Math.floor(secs / 3600)} ${units.h} ${Math.floor((secs % 3600) / 60)} ${units.m}`;
  if (secs >= 60)
    return `${Math.floor(secs / 60)} ${units.m} ${secs % 60} ${units.s}`;
  return `${secs} ${units.s}`;
}