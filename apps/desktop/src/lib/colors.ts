/** Deterministic muted palette for resources/categories. No business logic. */

const PALETTE = [
  '#4f9cf0', // blue
  '#46b08a', // green
  '#d9a13b', // amber
  '#b07cc6', // purple
  '#e07b5f', // terracotta
  '#5fc6c0', // teal
  '#e0c35f', // gold
  '#8fa3b8', // slate
];

function hashString(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

export function colorForResource(resourceId: string): string {
  return PALETTE[hashString(resourceId) % PALETTE.length];
}

export function colorForCategory(categoryId: string): string {
  return PALETTE[(hashString(`cat:${categoryId}`) + 3) % PALETTE.length];
}
