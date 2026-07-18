import { useState, type FormEvent } from 'react';
import type { Book, ScanJob } from '../domain';
import { formatDuration } from '../domain';
import './ScanProgress.css';

interface ScanProgressProps {
  book: Book | null;
  job: ScanJob | null;
  onPause: (jobId: string) => void;
  onResume: (jobId: string) => void;
}

const QUICK_COMMANDS = [
  '只看严重的雷点',
  '遇到疑似项先问我',
  '把当前依据说清楚',
];

function ScanConversationDemo({ book, job }: { book: Book; job: ScanJob }) {
  const [draft, setDraft] = useState('');
  const [lastCommand, setLastCommand] = useState<string | null>(null);

  const submitCommand = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const command = draft.trim();
    if (!command) return;
    setLastCommand(command);
    setDraft('');
  };

  const progressReply = job.status === 'running'
    ? `我正在读《${book.title}》，已经看到第 ${job.currentChapter} 章。命中会带着章节和摘录出现在右侧；拿不准的地方，我会先留作待确认。`
    : job.status === 'paused'
      ? `我把《${book.title}》停在第 ${job.currentChapter} 章了，已经找到的依据都还在。你准备好时再让我继续。`
      : job.status === 'completed'
        ? `《${book.title}》已经读完。你可以让我只看某类雷点，或者回头解释一条命中的依据。`
        : `《${book.title}》已经排进阅读列表，等扫描任务开始后我会在这里同步进展。`;

  return (
    <section className="scan-conversation" aria-labelledby="scan-conversation-title">
      <div className="scan-conversation__heading">
        <div>
          <p className="scan-conversation__eyebrow">交互方式预览</p>
          <h4 id="scan-conversation-title" className="scan-conversation__title">直接告诉我你在意什么</h4>
        </div>
        <span className="scan-conversation__demo-badge">界面演示</span>
      </div>

      <div className="scan-conversation__messages" aria-live="polite">
        <div className="scan-message scan-message--assistant">
          <span className="scan-message__speaker">扫文助手</span>
          <p>{progressReply}</p>
        </div>

        {lastCommand && (
          <>
            <div className="scan-message scan-message--user">
              <span className="scan-message__speaker">你</span>
              <p>{lastCommand}</p>
            </div>
            <div className="scan-message scan-message--assistant">
              <span className="scan-message__speaker">扫文助手</span>
              <p>记下了。这句话目前只留在界面演示里，还不会改变扫描任务；接上扫描引擎后，我会按你的要求调整本次阅读。</p>
            </div>
          </>
        )}
      </div>

      <div className="scan-conversation__suggestions" aria-label="可以试着这样说">
        {QUICK_COMMANDS.map(command => (
          <button
            key={command}
            type="button"
            className="scan-conversation__suggestion"
            onClick={() => setDraft(command)}
          >
            {command}
          </button>
        ))}
      </div>

      <form className="scan-command" onSubmit={submitCommand}>
        <label className="sr-only" htmlFor="scan-command-input">告诉扫文助手本次扫描要求</label>
        <textarea
          id="scan-command-input"
          className="scan-command__input"
          rows={2}
          maxLength={300}
          value={draft}
          onChange={event => setDraft(event.target.value)}
          placeholder="比如：只扫感情类雷点，遇到疑似项先停下来告诉我"
        />
        <button className="scan-command__submit" type="submit" disabled={!draft.trim()}>
          记下这句话
        </button>
      </form>
      <p className="scan-conversation__notice">
        这里暂时只演示自然语言交互，不会向模型发请求，也不会改动真实文件。
      </p>
    </section>
  );
}

export function ScanProgress({ book, job, onPause, onResume }: ScanProgressProps) {
  if (!book) {
    return (
      <section className="scan-progress scan-progress--empty" aria-label="扫描进度">
        <div className="scan-empty">
          <p className="scan-empty__icon" aria-hidden="true">
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="11" cy="11" r="8" />
              <line x1="21" y1="21" x2="16.65" y2="16.65" />
            </svg>
          </p>
          <h3 className="scan-empty__title">未选择书籍</h3>
          <p className="scan-empty__desc">
            请在左侧书架中选择一本书开始扫描，或先导入新的小说文件。
          </p>
        </div>
      </section>
    );
  }

  if (!job) {
    return (
      <section className="scan-progress scan-progress--idle" aria-label={`扫描进度 — ${book.title}`}>
        <div className="scan-idle">
          <h3 className="scan-idle__title">{book.title}</h3>
          <p className="scan-idle__author">{book.author}</p>
          <p className="scan-idle__meta">
            {book.totalChapters} 章 · {book.format.toUpperCase()}
          </p>
          <p className="scan-idle__desc">
            此书籍尚未开始扫描。请先在「规则」标签页中配置扫描规则，然后点击下方按钮开始扫描。
          </p>
          <button className="scan-start-btn" disabled aria-disabled="true" title="演示版本暂不支持启动扫描">
            开始扫描（演示版暂不可用）
          </button>
        </div>
      </section>
    );
  }

  // 运行中或暂停
  const isRunning = job.status === 'running';
  const isPaused = job.status === 'paused';
  const isCompleted = job.status === 'completed';
  const pct = Math.round(job.progress * 100);

  return (
    <section className="scan-progress" aria-label={`扫描进度 — ${book.title}`}>
      <div className="scan-header">
        <div className="scan-header__info">
          <h3 className="scan-header__title">{book.title}</h3>
          <p className="scan-header__author">{book.author}</p>
        </div>
        <span className={`scan-status-badge scan-status-badge--${job.status}`}>
          {isRunning ? '扫描中' : isPaused ? '已暂停' : isCompleted ? '已完成' : '准备中'}
        </span>
      </div>

      {/* 进度条 */}
      <div className="scan-bar-section">
        <div className="scan-bar" role="progressbar" aria-valuenow={pct} aria-valuemin={0} aria-valuemax={100} aria-label={`扫描进度 ${pct}%`}>
          <div
            className={`scan-bar__fill${isPaused ? ' scan-bar__fill--paused' : ''}${isCompleted ? ' scan-bar__fill--completed' : ''}`}
            style={{ width: `${pct}%` }}
          />
        </div>
        <span className="scan-bar__label">{pct}%</span>
      </div>

      {/* 统计 */}
      <div className="scan-stats">
        <div className="scan-stat">
          <span className="scan-stat__label">当前章节</span>
          <span className="scan-stat__value">{job.currentChapter} / {job.totalChapters}</span>
        </div>
        <div className="scan-stat">
          <span className="scan-stat__label">预估剩余</span>
          <span className="scan-stat__value">{formatDuration(job.estimatedRemaining)}</span>
        </div>
        <div className="scan-stat">
          <span className="scan-stat__label">前文记忆整理</span>
          <span className="scan-stat__value">{job.compressionCount} 次</span>
        </div>
      </div>

      {/* 上下文压缩提示 */}
      {job.compressionCount > 0 && job.lastCompressionAt && (
        <div className="scan-compression-hint" role="status" aria-live="polite">
          <span className="scan-compression-hint__icon" aria-hidden="true">[i]</span>
          <span>
            已整理过 {job.compressionCount} 次前文记忆（最近一次：
            {new Date(job.lastCompressionAt).toLocaleTimeString('zh-CN')}）。
            章节来源和仍待确认的线索会继续保留，后端接通后再验证长篇完整性。
          </span>
        </div>
      )}

      {/* 操作按钮 */}
      <div className="scan-actions">
        {isRunning && (
          <button
            className="scan-btn scan-btn--pause"
            onClick={() => onPause(job.id)}
            aria-label="暂停扫描"
          >
            暂停扫描
          </button>
        )}
        {isPaused && (
          <button
            className="scan-btn scan-btn--resume"
            onClick={() => onResume(job.id)}
            aria-label="继续扫描"
          >
            继续扫描
          </button>
        )}
      </div>

      <ScanConversationDemo book={book} job={job} />

      {/* 演示提示 */}
      <div className="scan-demo-hint">
        <span aria-hidden="true">[D]</span>
        <span>当前进度与命中均为原创演示数据。暂停和继续只改变本地界面状态，尚未连接真实扫描任务。</span>
      </div>
    </section>
  );
}
