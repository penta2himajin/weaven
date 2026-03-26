import "@testing-library/jest-dom/vitest";

// Mock ResizeObserver for React Flow (not available in jsdom).
global.ResizeObserver = class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
} as any;
