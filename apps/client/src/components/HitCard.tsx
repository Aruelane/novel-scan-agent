import type { Hit } from '../domain';
import { confidenceLabel, findingStatusLabel } from '../domain';
import './HitCard.css';

interface HitCardProps {
  hit: Hit;
  onConfirm: () => void;
  onFalsePositive: () => void;
}

export function HitCard({ hit, onConfirm, onFalsePositive }: HitCardProps) {
  const isReviewing = hit.reviewStatus === 'reviewing';
  const isConfirmed = hit.reviewStatus === 'confirmed';

  return (
    <article
      className={`hit-card${isConfirmed ? ' hit-card--confirmed' : ''}${!isReviewing && !isConfirmed ? ' hit-card--false' : ''}`}
      aria-label={`命中：${hit.ruleName}，第 ${hit.chapter} 章${hit.chapterTitle ? ` ${hit.chapterTitle}` : ''}`}
    >
      {/* 头部 */}
      <div className="hit-card__header">
        <span className="hit-card__rule-name">{hit.ruleName}</span>
        <span className={`hit-card__confidence hit-card__confidence--${hit.confidence}`}>
          {confidenceLabel(hit.confidence)}
        </span>
        <span className="hit-card__confidence">
          {hit.sourceKind === 'original_demo'
            ? '演示核验状态'
            : findingStatusLabel(hit.findingStatus)}
        </span>
      </div>

      {/* 位置信息 */}
      <div className="hit-card__location">
        <span className="hit-card__chapter">
          第 {hit.chapter} 章 {hit.chapterTitle}
        </span>
        <span className="hit-card__position">{hit.position}</span>
      </div>

      {/* 原文摘录 */}
      <blockquote className="hit-card__excerpt" cite={`第${hit.chapter}章 ${hit.chapterTitle}`}>
        <span className="hit-card__excerpt-label">
          {hit.sourceKind === 'original_demo' ? '项目原创演示摘录：' : '原文摘录：'}
        </span>
        {hit.excerpt}
      </blockquote>

      {/* 扫描理由 */}
      <details className="hit-card__reason">
        <summary className="hit-card__reason-summary">为什么标记这里</summary>
        <p className="hit-card__reason-text">{hit.reason}</p>
      </details>

      {/* 审核状态和操作 */}
      <div className="hit-card__actions">
        {isReviewing && (
          <div className="hit-card__review-btns">
            <button
              className="hit-card__btn hit-card__btn--confirm"
              onClick={onConfirm}
              aria-label={`确认命中：${hit.ruleName}`}
            >
              确认
            </button>
            <button
              className="hit-card__btn hit-card__btn--false"
              onClick={onFalsePositive}
              aria-label={`标记误报：${hit.ruleName}`}
            >
              误报
            </button>
          </div>
        )}
        {isConfirmed && (
          <span className="hit-card__review-status hit-card__review-status--confirmed">
            已确认
          </span>
        )}
        {!isReviewing && !isConfirmed && (
          <span className="hit-card__review-status hit-card__review-status--false">
            已标记误报
          </span>
        )}
      </div>
    </article>
  );
}
