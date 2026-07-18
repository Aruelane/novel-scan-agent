import type { Book, ScanJob } from '../domain';
import { bookStatusLabel, formatFileSize } from '../domain';
import './Sidebar.css';

interface SidebarProps {
  books: Book[];
  jobs: ScanJob[];
  selectedBookId: string | null;
  onSelectBook: (bookId: string) => void;
  className?: string;
}

export function Sidebar({ books, jobs, selectedBookId, onSelectBook, className }: SidebarProps) {
  const getJobForBook = (bookId: string) => jobs.find(j => j.bookId === bookId);

  return (
    <aside className={`sidebar ${className ?? ''}`} role="navigation" aria-label="书架与任务">
      <div className="sidebar-header">
        <h2 className="sidebar-title">书架</h2>
        <span className="sidebar-badge" aria-label={`共 ${books.length} 本书`}>
          {books.length}
        </span>
      </div>

      <ul className="book-list" role="list">
        {books.map(book => {
          const job = getJobForBook(book.id);
          const isSelected = book.id === selectedBookId;

          return (
            <li key={book.id}>
              <button
                className={`book-card${isSelected ? ' book-card--selected' : ''}`}
                onClick={() => onSelectBook(book.id)}
                aria-current={isSelected ? 'true' : undefined}
                aria-label={`${book.title}，作者 ${book.author}，${bookStatusLabel(book.status)}`}
              >
                <div className="book-card__main">
                  <span className="book-card__title">{book.title}</span>
                  <span className="book-card__author">{book.author}</span>
                </div>
                <div className="book-card__meta">
                  <span className="book-card__format">{book.format.toUpperCase()}</span>
                  <span className="book-card__size">{formatFileSize(book.fileSize)}</span>
                </div>
                {job && job.status === 'running' && (
                  <div className="book-card__progress">
                    <div className="book-card__progress-bar" role="progressbar" aria-valuenow={Math.round(job.progress * 100)} aria-valuemin={0} aria-valuemax={100}>
                      <div className="book-card__progress-fill" style={{ width: `${Math.round(job.progress * 100)}%` }} />
                    </div>
                    <span className="book-card__progress-text">{Math.round(job.progress * 100)}%</span>
                  </div>
                )}
                {job && job.status === 'paused' && (
                  <div className="book-card__progress">
                    <div className="book-card__progress-bar">
                      <div className="book-card__progress-fill book-card__progress-fill--paused" style={{ width: `${Math.round(job.progress * 100)}%` }} />
                    </div>
                    <span className="book-card__progress-text book-card__progress-text--paused">
                      {Math.round(job.progress * 100)}% (暂停)
                    </span>
                  </div>
                )}
                <div className="book-card__status">
                  <span className={`status-dot status-dot--${book.status}`} aria-hidden="true" />
                  {bookStatusLabel(book.status)}
                </div>
              </button>
            </li>
          );
        })}
      </ul>

      <div className="sidebar-footer">
        <span className="sidebar-footer__text">扫文助手 v0.1</span>
      </div>
    </aside>
  );
}
