import { describe, it, expect } from 'vitest';
import { demoBooks, demoRules, demoScanJobs, demoHits, demoProviders } from '../demo-data';
import { ALL_FORMATS } from '../domain';

describe('demoBooks', () => {
  it('contains at least one book', () => {
    expect(demoBooks.length).toBeGreaterThan(0);
  });

  it('each book has required fields', () => {
    for (const book of demoBooks) {
      expect(book.id).toBeTruthy();
      expect(book.title).toBeTruthy();
      expect(book.author).toBeTruthy();
      expect(book.format).toBeTruthy();
      expect(book.totalChapters).toBeGreaterThan(0);
      expect(book.fileSize).toBeGreaterThan(0);
    }
  });

  it('each book has a valid status', () => {
    const validStatuses = ['idle', 'scanning', 'scanned', 'failed'];
    for (const book of demoBooks) {
      expect(validStatuses).toContain(book.status);
    }
  });

  it('only shows demo books whose formats are currently importable', () => {
    const readyFormats = new Set(
      ALL_FORMATS
        .filter(format => format.status === 'ready')
        .map(format => format.format),
    );

    for (const book of demoBooks) {
      expect(readyFormats.has(book.format)).toBe(true);
    }
  });

  it('uses clearly labelled original demonstration books', () => {
    for (const book of demoBooks) {
      expect(book.title).toMatch(/^扫评演示·/);
      expect(book.author).toBe('界面演示文本, 非真实出版物');
      expect(book.sourceDisplayName).toBeTruthy();
      expect(book).not.toHaveProperty('filePath');
    }
  });
});

describe('demoRules', () => {
  it('contains both landmine and frustration rules', () => {
    const landmines = demoRules.filter(r => r.category === 'landmine');
    const frustrations = demoRules.filter(r => r.category === 'frustration');
    expect(landmines.length).toBeGreaterThan(0);
    expect(frustrations.length).toBeGreaterThan(0);
  });

  it('each rule has valid severity 1-5', () => {
    for (const rule of demoRules) {
      expect(rule.severity).toBeGreaterThanOrEqual(1);
      expect(rule.severity).toBeLessThanOrEqual(5);
    }
  });

  it('each rule has a name and description', () => {
    for (const rule of demoRules) {
      expect(rule.name).toBeTruthy();
      expect(rule.description).toBeTruthy();
    }
  });
});

describe('demoScanJobs', () => {
  it('has at least one job', () => {
    expect(demoScanJobs.length).toBeGreaterThan(0);
  });

  it('each job references a valid book id', () => {
    const bookIds = new Set(demoBooks.map(b => b.id));
    for (const job of demoScanJobs) {
      expect(bookIds.has(job.bookId)).toBe(true);
    }
  });

  it('each job has progress between 0 and 1', () => {
    for (const job of demoScanJobs) {
      expect(job.progress).toBeGreaterThanOrEqual(0);
      expect(job.progress).toBeLessThanOrEqual(1);
    }
  });
});

describe('demoHits', () => {
  it('has at least 2 hits as required', () => {
    expect(demoHits.length).toBeGreaterThanOrEqual(2);
  });

  it('each hit has chapter, position, and excerpt', () => {
    for (const hit of demoHits) {
      expect(hit.chapter).toBeGreaterThan(0);
      expect(hit.position).toBeTruthy();
      expect(hit.excerpt).toBeTruthy();
      expect(hit.reason).toBeTruthy();
      expect(hit.sourceKind).toBe('original_demo');
    }
  });

  it('keeps evidence status explicit and separate from confidence', () => {
    const validStatuses = ['suspected', 'pending_confirmation', 'confirmed', 'rejected'];
    for (const hit of demoHits) {
      expect(validStatuses).toContain(hit.findingStatus);
    }
    expect(demoHits.some(hit => hit.findingStatus === 'confirmed')).toBe(true);
    expect(demoHits.some(hit => hit.findingStatus === 'pending_confirmation')).toBe(true);
  });

  it('each hit references a valid rule id', () => {
    const ruleIds = new Set(demoRules.map(r => r.id));
    for (const hit of demoHits) {
      expect(ruleIds.has(hit.ruleId)).toBe(true);
    }
  });

  it('each hit references a valid job id', () => {
    const jobIds = new Set(demoScanJobs.map(j => j.id));
    for (const hit of demoHits) {
      expect(jobIds.has(hit.jobId)).toBe(true);
    }
  });
});

describe('demoProviders', () => {
  it('includes all five provider types', () => {
    const types = demoProviders.map(p => p.type);
    expect(types).toContain('openai');
    expect(types).toContain('anthropic');
    expect(types).toContain('gemini');
    expect(types).toContain('deepseek');
    expect(types).toContain('local');
  });

  it('each provider has endpoint defined', () => {
    for (const p of demoProviders) {
      expect(p.endpoint).toBeTruthy();
      // model may be empty when not yet selected
      expect(typeof p.model).toBe('string');
    }
  });

  it('exposes credential state without carrying key plaintext', () => {
    for (const provider of demoProviders) {
      expect(['missing', 'configured', 'unavailable']).toContain(provider.credentialState);
      expect('apiKey' in provider).toBe(false);
    }
  });
});

describe('cross-contamination guard', () => {
  it("switching books doesn't leak hits from a previous book", () => {
    // book-001 has job-001 -> hits hit-001 and hit-002.
    // book-002 has no job at all -> should have zero hits.
    const book001JobIds = demoScanJobs
      .filter(j => j.bookId === 'book-001')
      .map(j => j.id);
    const book002JobIds = demoScanJobs
      .filter(j => j.bookId === 'book-002')
      .map(j => j.id);

    const hitsForBook001 = demoHits.filter(h => book001JobIds.includes(h.jobId));
    const hitsForBook002 = demoHits.filter(h => book002JobIds.includes(h.jobId));

    // book-001 has 2 demo hits
    expect(hitsForBook001.length).toBe(2);

    // book-002 has no scan job so it should have zero hits
    expect(hitsForBook002.length).toBe(0);

    // No hit belongs to book-002 (cross-contamination guard)
    expect(demoHits.every(h => !book002JobIds.includes(h.jobId))).toBe(true);
  });

  it('book with no scan job has zero hits', () => {
    const bookIdsWithJobs = new Set(demoScanJobs.map(j => j.bookId));
    const booksWithoutJobs = demoBooks.filter(b => !bookIdsWithJobs.has(b.id));

    for (const book of booksWithoutJobs) {
      const jobIds = demoScanJobs
        .filter(j => j.bookId === book.id)
        .map(j => j.id);
      const hits = demoHits.filter(h => jobIds.includes(h.jobId));
      expect(hits).toHaveLength(0);
    }
  });

  it('all demo book names contain the project identifier', () => {
    for (const book of demoBooks) {
      expect(book.title).toMatch(/^扫评演示·/);
    }
  });
});
