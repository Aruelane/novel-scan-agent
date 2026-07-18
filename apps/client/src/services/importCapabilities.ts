import { invoke, isTauri } from '@tauri-apps/api/core';
import {
  ALL_FORMATS,
  type BookFormat,
  type FormatInfo,
  type FormatStatus,
} from '../domain';

const KNOWN_FORMATS = new Set<BookFormat>([
  'epub',
  'pdf',
  'doc',
  'docx',
  'txt',
  'markdown',
  'html',
  'mobi',
  'azw3',
  'zip',
  '7z',
]);

const KNOWN_STATUSES = new Set<FormatStatus>([
  'ready',
  'pending',
  'unsupported',
]);

export const IMPORT_CAPABILITIES_FALLBACK_NOTICE =
  '暂时无法读取当前设备的格式能力，已显示内置能力清单。';

export interface ImportCapabilitiesLoadResult {
  formats: FormatInfo[];
  source: 'native' | 'static';
  notice: string | null;
}

function cloneStaticFormats(): FormatInfo[] {
  return ALL_FORMATS.map(format => ({
    ...format,
    extensions: [...format.extensions],
  }));
}

function requireRecord(value: unknown): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new TypeError('Invalid import capability entry');
  }
  return value as Record<string, unknown>;
}

function requireString(value: unknown, field: string): string {
  if (typeof value !== 'string' || value.trim().length === 0) {
    throw new TypeError(`Invalid import capability ${field}`);
  }
  return value.trim();
}

function mapFormat(value: unknown): BookFormat {
  const format = requireString(value, 'formatId');
  if (!KNOWN_FORMATS.has(format as BookFormat)) {
    throw new TypeError('Unknown import capability format');
  }
  return format as BookFormat;
}

function mapStatus(value: unknown): FormatStatus {
  const status = requireString(value, 'status');
  if (!KNOWN_STATUSES.has(status as FormatStatus)) {
    throw new TypeError('Unknown import capability status');
  }
  return status as FormatStatus;
}

function mapExtensions(value: unknown): string[] {
  if (!Array.isArray(value) || value.length === 0) {
    throw new TypeError('Invalid import capability extensions');
  }

  return value.map(extension => {
    const normalized = requireString(extension, 'extension')
      .replace(/^\.+/, '')
      .toLocaleLowerCase('en-US');
    if (normalized.length === 0) {
      throw new TypeError('Invalid import capability extension');
    }
    return `.${normalized}`;
  });
}

/**
 * Converts the native command payload at the trust boundary. Unknown formats
 * and states are rejected so the UI never silently invents support.
 */
export function mapImportCapabilities(payload: unknown): FormatInfo[] {
  if (!Array.isArray(payload)) {
    throw new TypeError('Invalid import capabilities response');
  }

  if (payload.length === 0) {
    throw new TypeError('Import capabilities response is empty');
  }

  const seenFormats = new Set<BookFormat>();
  return payload.map(entry => {
    const record = requireRecord(entry);
    const format = mapFormat(record.formatId);
    if (seenFormats.has(format)) {
      throw new TypeError('Duplicate import capability format');
    }
    seenFormats.add(format);

    return {
      format,
      label: requireString(record.label, 'label'),
      extensions: mapExtensions(record.extensions),
      status: mapStatus(record.status),
      note: requireString(record.detail, 'detail'),
    };
  });
}

/**
 * Browser previews stay fully usable without a native shell. A native bridge
 * failure is deliberately reduced to a fixed, path-free notice and the honest
 * built-in registry.
 */
export async function loadImportCapabilities(): Promise<ImportCapabilitiesLoadResult> {
  if (!isTauri()) {
    return { formats: cloneStaticFormats(), source: 'static', notice: null };
  }

  try {
    const payload = await invoke<unknown>('import_capabilities');
    return {
      formats: mapImportCapabilities(payload),
      source: 'native',
      notice: null,
    };
  } catch {
    return {
      formats: cloneStaticFormats(),
      source: 'static',
      notice: IMPORT_CAPABILITIES_FALLBACK_NOTICE,
    };
  }
}
