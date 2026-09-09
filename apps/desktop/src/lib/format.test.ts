import { describe, expect, it } from 'vitest';
import { formatAgo, formatDuration, formatGold, formatInt, formatPct, formatQty, formatTime } from './format';

describe('formatGold (copper → gold)', () => {
  it('formats zero and small values', () => {
    expect(formatGold(0)).toBe('0.00g');
    expect(formatGold(12_345)).toBe('1.23g');
  });
  it('switches precision by magnitude', () => {
    expect(formatGold(1_234_567)).toBe('123.5g');
    expect(formatGold(12_000_000)).toBe('1,200g');
  });
  it('handles nullish input', () => {
    expect(formatGold(null)).toBe('—');
    expect(formatGold(undefined)).toBe('—');
    expect(formatGold(NaN)).toBe('—');
  });
});

describe('formatDuration', () => {
  it('formats seconds, minutes, hours', () => {
    expect(formatDuration(45)).toBe('45s');
    expect(formatDuration(125)).toBe('2m 05s');
    expect(formatDuration(3700)).toBe('1h 01m');
  });
  it('handles invalid input', () => {
    expect(formatDuration(null)).toBe('—');
    expect(formatDuration(-5)).toBe('—');
  });
});

describe('format helpers', () => {
  it('formats ints and quantities', () => {
    expect(formatInt(1234567)).toBe('1,234,567');
    expect(formatQty(3.14159)).toBe('3.14');
    expect(formatQty(150.7)).toBe('151');
  });
  it('formats percentages', () => {
    expect(formatPct(0.754)).toBe('75%');
    expect(formatPct(null)).toBe('—');
  });
  it('formats timestamps', () => {
    expect(formatTime(null)).toBe('never');
    expect(formatTime(0)).toBe('never');
    expect(formatTime(1_700_000_000)).toContain('2023');
  });
  it('formats relative age deterministically', () => {
    const now = 1_700_000_000_000;
    expect(formatAgo(1_700_000_000 - 30, now)).toBe('30s ago');
    expect(formatAgo(1_700_000_000 - 120, now)).toBe('2m ago');
    expect(formatAgo(1_700_000_000 - 7200, now)).toBe('2h ago');
    expect(formatAgo(1_700_000_000 - 200_000, now)).toBe('2d ago');
    expect(formatAgo(null)).toBe('never');
  });
});
