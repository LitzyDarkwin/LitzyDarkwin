import { afterEach } from 'vitest';
import { cleanup } from '@testing-library/react';

// jsdom render cleanup between tests (vitest runs without globals).
afterEach(() => {
  cleanup();
});
