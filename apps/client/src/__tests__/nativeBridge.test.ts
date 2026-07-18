import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke, mockIsTauri } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockIsTauri: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
  isTauri: mockIsTauri,
}));

// Re-import after mocking
import {
  loadImportCapabilities,
  mapImportCapabilities,
  IMPORT_CAPABILITIES_FALLBACK_NOTICE,
} from '../services/importCapabilities';

function capability(overrides: Record<string, unknown> = {}) {
  return {
    formatId: 'txt',
    label: '纯文本',
    extensions: ['txt'],
    mediaTypes: ['text/plain'],
    status: 'ready',
    detail: '支持 UTF-8 文本',
    sourceLocator: '章节 + 行号',
    coreDocumentFormat: 'plain_text',
    ...overrides,
  };
}

describe('mapImportCapabilities', () => {
  it('throws on empty array instead of returning silent success', () => {
    expect(() => mapImportCapabilities([])).toThrow(
      /Import capabilities response is empty/,
    );
  });

  it('throws on non-array payload', () => {
    expect(() => mapImportCapabilities(null)).toThrow(/Invalid/);
    expect(() => mapImportCapabilities('string')).toThrow(/Invalid/);
    expect(() => mapImportCapabilities(42)).toThrow(/Invalid/);
  });

  it('throws on duplicate format', () => {
    expect(() =>
      mapImportCapabilities([
        capability({ formatId: 'txt' }),
        capability({ formatId: 'txt' }),
      ]),
    ).toThrow(/Duplicate/);
  });

  it('throws on illegal/unknown format entry', () => {
    expect(() =>
      mapImportCapabilities([capability({ formatId: 'pages' })]),
    ).toThrow(/Unknown import capability format/);
  });
});

describe('loadImportCapabilities', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('isTauri()=false returns static fallback with no notice', async () => {
    mockIsTauri.mockReturnValue(false);

    const result = await loadImportCapabilities();

    expect(result.source).toBe('static');
    expect(result.notice).toBeNull();
    expect(result.formats.length).toBeGreaterThan(0);
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it('isTauri()=true + invoke success returns native result', async () => {
    mockIsTauri.mockReturnValue(true);
    mockInvoke.mockResolvedValue([
      capability({ formatId: 'txt', label: 'Pure Text', extensions: ['txt'], status: 'ready', detail: 'Supports UTF-8' }),
    ]);

    const result = await loadImportCapabilities();

    expect(result.source).toBe('native');
    expect(result.notice).toBeNull();
    expect(result.formats[0].label).toBe('Pure Text');
  });

  it('invoke rejects -> static fallback with notice', async () => {
    mockIsTauri.mockReturnValue(true);
    mockInvoke.mockRejectedValue(new Error('Connection refused'));

    const result = await loadImportCapabilities();

    expect(result.source).toBe('static');
    expect(result.notice).toBe(IMPORT_CAPABILITIES_FALLBACK_NOTICE);
  });

  it('notice does not leak across calls', async () => {
    // First call: invoke fails
    mockIsTauri.mockReturnValue(true);
    mockInvoke.mockRejectedValueOnce(new Error('fail'));

    const result1 = await loadImportCapabilities();
    expect(result1.notice).toBe(IMPORT_CAPABILITIES_FALLBACK_NOTICE);

    // Second call: invoke succeeds
    mockInvoke.mockResolvedValueOnce([
      capability(),
    ]);

    const result2 = await loadImportCapabilities();
    expect(result2.notice).toBeNull();
    expect(result2.source).toBe('native');
  });

  it('notice does not contain raw error messages or file paths', async () => {
    mockIsTauri.mockReturnValue(true);
    mockInvoke.mockRejectedValue(new Error('Failed at C:\\secret\\path'));

    const result = await loadImportCapabilities();

    expect(result.notice).toBe(IMPORT_CAPABILITIES_FALLBACK_NOTICE);
    expect(result.notice).not.toContain('C:');
    expect(result.notice).not.toContain('secret');
    expect(result.notice).not.toContain('Failed');
    expect(result.notice).not.toContain('path');
  });
});
