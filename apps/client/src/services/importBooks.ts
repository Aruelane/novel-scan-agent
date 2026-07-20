import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Book, BookFormat, FormatStatus } from '../domain';

/** DTO returned by the Rust import_novel_bytes command. */
interface ImportResultDto {
  book_id: string;
  display_name: string;
  format: string;
  chapter_count: number;
  character_count: number;
  warnings: string[];
}

function mapFormat(format: string): BookFormat {
  const valid = new Set<BookFormat>([
    'epub', 'pdf', 'docx', 'txt', 'markdown', 'html', 'mobi', 'azw3', 'zip', '7z', 'doc',
  ]);
  if (valid.has(format as BookFormat)) return format as BookFormat;
  return 'txt'; // safe fallback
}

export interface ImportBookResult {
  book: Book;
  summary: string;
}

/**
 * Import a book via the Tauri native command. Only available in Tauri context.
 * Browser previews should use the static notice instead.
 */
export async function importBookBytes(
  sourceName: string,
  bytes: Uint8Array,
): Promise<ImportBookResult> {
  if (!isTauri()) {
    throw new Error('文件导入仅在桌面应用中可用');
  }

  const dto = await invoke<ImportResultDto>('import_novel_bytes', {
    sourceName,
    bytes: Array.from(bytes),
  });

  const format = mapFormat(dto.format);

  const book: Book = {
    id: dto.book_id,
    title: dto.display_name,
    author: '',
    format,
    sourceDisplayName: dto.display_name,
    status: 'idle' as const,
    addedAt: new Date().toISOString(),
    totalChapters: dto.chapter_count,
    fileSize: bytes.byteLength,
  };

  const summary =
    `导入完成：《${dto.display_name}》` +
    `\n格式：${format.toUpperCase()}，共 ${dto.chapter_count} 章，${dto.character_count} 字` +
    (dto.warnings.length > 0 ? `\n注意：${dto.warnings.join('；')}` : '');

  return { book, summary };
}
