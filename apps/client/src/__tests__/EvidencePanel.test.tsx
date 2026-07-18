import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { EvidencePanel } from '../components/EvidencePanel';
import type { Hit, ScanJob } from '../domain';

const demoJobs: ScanJob[] = [
  {
    id: 'job-001',
    bookId: 'book-001',
    ruleIds: ['rule-001', 'rule-002'],
    status: 'running',
    progress: 0.64,
    currentChapter: 243,
    totalChapters: 380,
    startedAt: '2026-07-17T16:00:00Z',
    compressionCount: 2,
    lastCompressionAt: '2026-07-17T16:12:30Z',
    estimatedRemaining: 185,
  },
];

const demoHits: Hit[] = [
  {
    id: 'hit-001',
    jobId: 'job-001',
    ruleId: 'rule-002',
    ruleName: '虐主 / 憋屈',
    chapter: 87,
    chapterTitle: '无人回信',
    position: '第 34 段',
    sourceKind: 'original_demo',
    excerpt: '测试摘录1',
    reason: '测试理由1',
    confidence: 'high',
    findingStatus: 'confirmed',
    reviewStatus: 'confirmed',
    foundAt: '2026-07-17T16:03:15Z',
  },
  {
    id: 'hit-002',
    jobId: 'job-001',
    ruleId: 'rule-007',
    ruleName: '主角智商掉线',
    chapter: 156,
    chapterTitle: '最后一盏灯',
    position: '第 12 段',
    sourceKind: 'original_demo',
    excerpt: '测试摘录2',
    reason: '测试理由2',
    confidence: 'medium',
    findingStatus: 'pending_confirmation',
    reviewStatus: 'reviewing',
    foundAt: '2026-07-17T16:08:42Z',
  },
  // Orphan hit: no matching job
  {
    id: 'hit-orphan',
    jobId: 'job-nonexistent',
    ruleId: 'rule-001',
    ruleName: '绿帽 / NTR',
    chapter: 10,
    chapterTitle: '测试章',
    position: '第 1 段',
    sourceKind: 'original_demo',
    excerpt: 'orphan excerpt',
    reason: 'no reason',
    confidence: 'low',
    findingStatus: 'suspected',
    reviewStatus: 'reviewing',
    foundAt: '2026-07-17T16:00:00Z',
  },
];

const noop = () => {};

describe('EvidencePanel', () => {
  it('shows empty state when selectedBookId is null', () => {
    render(
      <EvidencePanel
        hits={demoHits}
        jobs={demoJobs}
        selectedBookId={null}
        onUpdateReview={noop}
      />,
    );

    expect(screen.getByText(/未选择书籍/)).toBeTruthy();
  });

  it('shows 2 hits for book-001 which has a scan job', () => {
    render(
      <EvidencePanel
        hits={demoHits}
        jobs={demoJobs}
        selectedBookId="book-001"
        onUpdateReview={noop}
      />,
    );

    // Should show 2命中 (hit-001 and hit-002 belong to job-001 -> book-001)
    expect(screen.getByText(/2 条命中/)).toBeTruthy();
    expect(screen.getByText(/虐主/)).toBeTruthy();
    expect(screen.getByText(/主角智商掉线/)).toBeTruthy();
  });

  it('shows 0 hits for book-002 which has no scan job', () => {
    render(
      <EvidencePanel
        hits={demoHits}
        jobs={demoJobs}
        selectedBookId="book-002"
        onUpdateReview={noop}
      />,
    );

    expect(screen.getByText(/此书尚未启动扫描/)).toBeTruthy();
  });

  it('shows 0 hits for book-003 (unknown bookId, no job)', () => {
    render(
      <EvidencePanel
        hits={demoHits}
        jobs={demoJobs}
        selectedBookId="book-003"
        onUpdateReview={noop}
      />,
    );

    expect(screen.getByText(/此书尚未启动扫描/)).toBeTruthy();
  });

  it('switching from book-001 to book-002 clears old evidence', () => {
    const { rerender } = render(
      <EvidencePanel
        hits={demoHits}
        jobs={demoJobs}
        selectedBookId="book-001"
        onUpdateReview={noop}
      />,
    );

    expect(screen.getByText(/2 条命中/)).toBeTruthy();

    rerender(
      <EvidencePanel
        hits={demoHits}
        jobs={demoJobs}
        selectedBookId="book-002"
        onUpdateReview={noop}
      />,
    );

    expect(screen.getByText(/此书尚未启动扫描/)).toBeTruthy();
  });

  it('does not display orphan hits (hits with no matching job)', () => {
    render(
      <EvidencePanel
        hits={demoHits}
        jobs={demoJobs}
        selectedBookId="book-001"
        onUpdateReview={noop}
      />,
    );

    // The orphan hit (hit-orphan) should not appear for book-001
    expect(screen.queryByText(/绿帽/)).toBeNull();
  });

  it('sourceKind original_demo with confirmed findingStatus does NOT count toward real confirmed count', () => {
    render(
      <EvidencePanel
        hits={demoHits}
        jobs={demoJobs}
        selectedBookId="book-001"
        onUpdateReview={noop}
      />,
    );

    // hit-001 is original_demo + confirmed → counts as demo confirmed
    // hit-002 is original_demo + pending_confirmation → counts as reviewing
    // Real confirmed should be 0, demo confirmed should be 1
    expect(screen.getByText(/0 原文核验/)).toBeTruthy();
    expect(screen.getByText(/1 演示核验/)).toBeTruthy();
  });

  it('displays "演示核验状态" label for demo confirmed hits', () => {
    render(
      <EvidencePanel
        hits={demoHits}
        jobs={demoJobs}
        selectedBookId="book-001"
        onUpdateReview={noop}
      />,
    );

    // The HitCard for hit-001 (original_demo + confirmed) should show 演示核验状态
    const labels = screen.getAllByText('演示核验状态');
    expect(labels.length).toBeGreaterThanOrEqual(1);
  });

  it('null selectedBookId shows empty hits, not all hits', () => {
    render(
      <EvidencePanel
        hits={demoHits}
        jobs={demoJobs}
        selectedBookId={null}
        onUpdateReview={noop}
      />,
    );

    // When null, it should show the "no book selected" message, not hits
    expect(screen.getByText(/未选择书籍/)).toBeTruthy();
    // None of the hit content should be visible
    expect(screen.queryByText(/虐主/)).toBeNull();
    expect(screen.queryByText(/主角智商掉线/)).toBeNull();
  });
});
