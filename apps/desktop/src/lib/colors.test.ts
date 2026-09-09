import { describe, expect, it } from 'vitest';
import { colorForCategory, colorForResource } from './colors';

describe('colors', () => {
  it('is deterministic per id', () => {
    expect(colorForResource('mycobloom')).toBe(colorForResource('mycobloom'));
    expect(colorForCategory('mining')).toBe(colorForCategory('mining'));
  });

  it('returns valid hex colors spread across ids', () => {
    const ids = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i'];
    const colors = new Set(ids.map(colorForResource));
    expect(colors.size).toBeGreaterThan(1);
    for (const c of colors) expect(c).toMatch(/^#[0-9a-f]{6}$/);
  });
});
