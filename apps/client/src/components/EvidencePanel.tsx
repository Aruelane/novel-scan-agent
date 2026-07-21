import type { Hit, ScanJob } from '../domain';
import { HitCard } from './HitCard';
import './EvidencePanel.css';

interface EvidencePanelProps {
  hits: Hit[];
  jobs: ScanJob[];
  selectedBookId: string | null;
  onUpdateReview: (hitId: string, status: 'confirmed' | 'false_positive') => void;
  className?: string;
}

export function EvidencePanel({ hits, jobs, selectedBookId, onUpdateReview, className }: EvidencePanelProps) {
  // Filter hits to only show those belonging to the currently selected book.
  // A hit is associated with a book through its job -> bookId chain.
  // When no book is selected, show nothing (never all hits).
  const bookJobIds = new Set(
    jobs
      .filter(j => selectedBookId !== null && j.bookId === selectedBookId)
      .map(j => j.id),
  );
  const relevantHits = selectedBookId !== null
    ? hits.filter(h => bookJobIds.has(h.jobId))
    : [];

  const confirmedCount = relevantHits.filter(
    h => h.findingStatus === 'confirmed',
  ).length;

  const reviewingCount = relevantHits.filter(h =>
    h.reviewStatus === 'reviewing'
    && (h.findingStatus === 'pending_confirmation' || h.findingStatus === 'suspected')
  ).length;

  const hasJob = selectedBookId !== null
    && jobs.some(j => j.bookId === selectedBookId);

  return (
    <aside className={`evidence-panel ${className ?? ''}`} aria-label="命中证据面板">
      <div className="evidence-panel__header">
        <h2 className="evidence-panel__title">命中结果</h2>
        <div className="evidence-panel__summary">
          <span className="evidence-panel__count" aria-label={`${relevantHits.length} 条命中`}>
            {relevantHits.length} 条命中
          </span>
          <span className="evidence-panel__detail">
            {confirmedCount} 已确认 · {reviewingCount} 待确认
          </span>
        </div>
      </div>

      {relevantHits.length === 0 ? (
        selectedBookId === null ? (
          <div className="evidence-panel__empty">
            <p className="evidence-panel__empty-icon" aria-hidden="true">
              <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M9 12l2 2 4-4" />
                <circle cx="12" cy="12" r="10" />
              </svg>
            </p>
            <p>未选择书籍。请从书架中选择一本书以查看命中结果。</p>
          </div>
        ) : !hasJob ? (
          <div className="evidence-panel__empty">
            <p className="evidence-panel__empty-icon" aria-hidden="true">
              <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="12" cy="12" r="10" />
                <path d="M10 8l6 4-6 4V8z" />
              </svg>
            </p>
            <p>此书尚未启动扫描。导入完成后请在「扫描」标签页启动。</p>
          </div>
        ) : (
          <div className="evidence-panel__empty">
            <p className="evidence-panel__empty-icon" aria-hidden="true">
              <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M9 12l2 2 4-4" />
                <circle cx="12" cy="12" r="10" />
              </svg>
            </p>
            <p>此书的扫描尚未产生命中。扫描进行中或完成后，命中项将在此显示。</p>
          </div>
        )
      ) : (
        <ul className="evidence-panel__list" role="list">
          {relevantHits.map(hit => (
            <li key={hit.id}>
              <HitCard
                hit={hit}
                onConfirm={() => onUpdateReview(hit.id, 'confirmed')}
                onFalsePositive={() => onUpdateReview(hit.id, 'false_positive')}
              />
            </li>
          ))}
        </ul>
      )}

      {relevantHits.length > 0 && (
        <div className="evidence-panel__footer">
          <span>点击确认 / 误报以标记审核结果</span>
        </div>
      )}
    </aside>
  );
}
