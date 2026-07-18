import { describe, expect, it } from 'vitest';
import { ALL_FORMATS } from '../domain';
import {
  loadImportCapabilities,
  mapImportCapabilities,
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
  it('preserves legacy DOC as explicitly unsupported', () => {
    const [mapped] = mapImportCapabilities([
      capability({
        formatId: 'doc',
        label: '旧版 Word DOC',
        extensions: ['doc'],
        status: 'unsupported',
        detail: '请另存为其他格式',
        coreDocumentFormat: 'other',
      }),
    ]);

    expect(mapped).toEqual({
      format: 'doc',
      label: '旧版 Word DOC',
      extensions: ['.doc'],
      status: 'unsupported',
      note: '请另存为其他格式',
    });
  });

  it('adds exactly one dot and normalizes extension casing', () => {
    const [mapped] = mapImportCapabilities([
      capability({ extensions: ['txt', '.TXT', '..Md'] }),
    ]);

    expect(mapped.extensions).toEqual(['.txt', '.txt', '.md']);
  });

  it('rejects an unknown format instead of claiming support', () => {
    expect(() => mapImportCapabilities([
      capability({ formatId: 'pages' }),
    ])).toThrow(/Unknown import capability format/);
  });

  it('rejects an unknown status instead of guessing', () => {
    expect(() => mapImportCapabilities([
      capability({ status: 'experimental' }),
    ])).toThrow(/Unknown import capability status/);
  });
});

describe('loadImportCapabilities', () => {
  it('keeps the browser demo on an isolated copy of the static registry', async () => {
    const result = await loadImportCapabilities();

    expect(result.source).toBe('static');
    expect(result.notice).toBeNull();
    expect(result.formats).toEqual(ALL_FORMATS);
    expect(result.formats).not.toBe(ALL_FORMATS);
    expect(result.formats[0].extensions).not.toBe(ALL_FORMATS[0].extensions);
  });
});
