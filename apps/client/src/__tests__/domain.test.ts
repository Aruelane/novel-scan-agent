/* ============================================================
 * domain.test.ts — 纯函数单元测试
 * ============================================================ */

import { describe, it, expect } from 'vitest';
import {
  ALL_FORMATS,
  formatFileSize,
  formatDuration,
  confidenceLabel,
  severityLabel,
  bookStatusLabel,
  findingStatusLabel,
  scanStatusLabel,
  categoryLabel,
} from '../domain';

describe('ALL_FORMATS', () => {
  it('marks all 6 implemented formats ready', () => {
    const ready = ALL_FORMATS.filter(format => format.status === 'ready').map(format => format.format);
    expect(ready).toEqual(['txt', 'markdown', 'html', 'epub', 'docx', 'pdf']);
  });

  it('keeps recognised but unfinished formats pending', () => {
    const pending = ALL_FORMATS.filter(format => format.status === 'pending').map(format => format.format);
    expect(pending).toEqual(['mobi', 'azw3', 'zip', '7z']);
  });

  it('does not claim legacy DOC support', () => {
    expect(ALL_FORMATS.find(format => format.format === 'doc')?.status).toBe('unsupported');
  });
});

describe('formatFileSize', () => {
  it('returns bytes for values under 1024', () => {
    expect(formatFileSize(0)).toBe('0 B');
    expect(formatFileSize(500)).toBe('500 B');
    expect(formatFileSize(1023)).toBe('1023 B');
  });

  it('returns KB for values under 1 MB', () => {
    expect(formatFileSize(1024)).toBe('1.0 KB');
    expect(formatFileSize(1536)).toBe('1.5 KB');
  });

  it('returns MB for values 1 MB and above', () => {
    expect(formatFileSize(1048576)).toBe('1.0 MB');
    expect(formatFileSize(5242880)).toBe('5.0 MB');
  });
});

describe('formatDuration', () => {
  it('returns seconds for values under 60', () => {
    expect(formatDuration(0)).toBe('0 秒');
    expect(formatDuration(0.2)).toBe('1 秒');
    expect(formatDuration(30)).toBe('30 秒');
    expect(formatDuration(59)).toBe('59 秒');
  });

  it('returns minutes and seconds for values under 1 hour', () => {
    expect(formatDuration(60)).toBe('1 分 0 秒');
    expect(formatDuration(60.2)).toBe('1 分 1 秒');
    expect(formatDuration(125)).toBe('2 分 5 秒');
  });

  it('returns hours and minutes for values 1 hour and above', () => {
    expect(formatDuration(3600)).toBe('1 时 0 分');
    expect(formatDuration(3700)).toBe('1 时 2 分');
  });
});

describe('bookStatusLabel', () => {
  it('keeps book lifecycle separate from scan job status', () => {
    expect(bookStatusLabel('idle')).toBe('待扫描');
    expect(bookStatusLabel('scanning')).toBe('扫描中');
    expect(bookStatusLabel('scanned')).toBe('已扫描');
    expect(bookStatusLabel('failed')).toBe('失败');
  });
});

describe('findingStatusLabel', () => {
  it('does not conflate evidence status with model confidence', () => {
    expect(findingStatusLabel('suspected')).toBe('疑似线索');
    expect(findingStatusLabel('pending_confirmation')).toBe('待后文确认');
    expect(findingStatusLabel('confirmed')).toBe('原文已核验');
    expect(findingStatusLabel('rejected')).toBe('已排除');
  });
});

describe('confidenceLabel', () => {
  it('returns correct Chinese labels', () => {
    expect(confidenceLabel('high')).toBe('高置信');
    expect(confidenceLabel('medium')).toBe('中置信');
    expect(confidenceLabel('low')).toBe('低置信');
  });
});

describe('severityLabel', () => {
  it('returns correct labels for all 5 levels', () => {
    expect(severityLabel(1)).toBe('轻微');
    expect(severityLabel(2)).toBe('较低');
    expect(severityLabel(3)).toBe('中等');
    expect(severityLabel(4)).toBe('较重');
    expect(severityLabel(5)).toBe('严重');
  });
});

describe('scanStatusLabel', () => {
  it('returns correct labels', () => {
    expect(scanStatusLabel('pending')).toBe('等待中');
    expect(scanStatusLabel('running')).toBe('扫描中');
    expect(scanStatusLabel('paused')).toBe('已暂停');
    expect(scanStatusLabel('completed')).toBe('已完成');
    expect(scanStatusLabel('failed')).toBe('失败');
  });
});

describe('categoryLabel', () => {
  it('returns correct labels', () => {
    expect(categoryLabel('landmine')).toBe('雷点');
    expect(categoryLabel('frustration')).toBe('郁闷点');
  });
});
