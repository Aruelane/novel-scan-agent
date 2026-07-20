import { useState, useRef } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import type { FormatInfo, FormatStatus } from '../domain';
import './ImportPanel.css';

const STATUS_ICON: Record<FormatStatus, string> = {
  ready: '[✓]',
  pending: '[·]',
  unsupported: '[—]',
};

const STATUS_LABEL: Record<FormatStatus, string> = {
  ready: '可导入',
  pending: '正在接入',
  unsupported: '暂不支持',
};

interface ImportPanelProps {
  formats: FormatInfo[];
  loading: boolean;
  notice: string | null;
  importError: string | null;
  onImport: (sourceName: string, bytes: Uint8Array) => Promise<string>;
  onClearError: () => void;
}

export function ImportPanel({
  formats,
  loading,
  notice,
  importError,
  onImport,
  onClearError,
}: ImportPanelProps) {
  const [importing, setImporting] = useState(false);
  const [summary, setSummary] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const isNative = isTauri();

  const handleFileChange = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    setImporting(true);
    setSummary(null);
    onClearError();

    try {
      const buffer = await file.arrayBuffer();
      const bytes = new Uint8Array(buffer);
      const result = await onImport(file.name, bytes);
      setSummary(result);
    } catch {
      // error is handled via importError prop
    } finally {
      setImporting(false);
      // Reset input so the same file can be re-selected
      if (fileInputRef.current) {
        fileInputRef.current.value = '';
      }
    }
  };

  const handleDropzoneClick = () => {
    if (isNative && !importing) {
      fileInputRef.current?.click();
    }
  };

  return (
    <section className="import-panel" aria-label="导入小说文件">
      <h3 className="import-panel__title">把小说交给我</h3>
      <p className="import-panel__desc">
        支持 TXT、Markdown、HTML、EPUB、DOCX 和文本型 PDF。
        选择文件后自动解析章节，为扫描做好准备。
      </p>

      {/* File input (hidden, triggered by dropzone click) */}
      <input
        ref={fileInputRef}
        type="file"
        accept=".txt,.md,.markdown,.mdown,.mkd,.html,.htm,.xhtml,.epub,.docx,.pdf"
        onChange={handleFileChange}
        style={{ display: 'none' }}
        aria-hidden="true"
      />

      {/* Dropzone */}
      <div
        className={`import-dropzone${isNative ? ' import-dropzone--active' : ''}${importing ? ' import-dropzone--busy' : ''}`}
        aria-label={
          isNative
            ? '点击选择小说文件'
            : '文件选择区域，当前为浏览器预览模式'
        }
        role={isNative ? 'button' : undefined}
        tabIndex={isNative ? 0 : undefined}
        aria-disabled={!isNative || importing}
        onClick={handleDropzoneClick}
        onKeyDown={(event) => {
          if (isNative && (event.key === 'Enter' || event.key === ' ')) {
            event.preventDefault();
            handleDropzoneClick();
          }
        }}
        onDragOver={(event) => { event.preventDefault(); }}
        onDrop={(event) => {
          event.preventDefault();
          if (!isNative || importing) return;
          const file = event.dataTransfer.files?.[0];
          if (file && fileInputRef.current) {
            // Manual trigger since we can't set FileList directly
            const dt = new DataTransfer();
            dt.items.add(file);
            fileInputRef.current.files = dt.files;
            fileInputRef.current.dispatchEvent(new Event('change', { bubbles: true }));
          }
        }}
      >
        <div className="import-dropzone__icon" aria-hidden="true">
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="17 8 12 3 7 8" />
            <line x1="12" y1="3" x2="12" y2="15" />
          </svg>
        </div>
        {importing ? (
          <p className="import-dropzone__text">正在导入…</p>
        ) : isNative ? (
          <>
            <p className="import-dropzone__text">
              点击选择文件，或拖拽小说文件到此处
            </p>
            <p className="import-dropzone__hint">
              支持 TXT / Markdown / HTML / EPUB / DOCX / PDF
            </p>
          </>
        ) : (
          <>
            <p className="import-dropzone__text">
              导入功能在桌面应用中可用
            </p>
            <p className="import-dropzone__hint">
              当前为浏览器预览，未连接桌面外壳。请使用 Tauri 桌面版进行真实导入。
            </p>
          </>
        )}
      </div>

      {/* Import result */}
      {summary && (
        <div className="import-result" role="status" aria-live="polite">
          <pre className="import-result__text">{summary}</pre>
        </div>
      )}

      {/* Import error */}
      {importError && (
        <div className="import-error" role="alert">
          <p className="import-error__text">{importError}</p>
          <button className="import-error__dismiss" onClick={onClearError} aria-label="关闭错误提示">
            ✕
          </button>
        </div>
      )}

      {/* Format capability table */}
      <div className="import-formats" aria-label="文件格式接入状态">
        <div className="import-formats__heading">
          <h4 className="import-formats__title">格式接入状态</h4>
          {loading && (
            <span className="import-formats__loading" role="status">
              正在确认当前设备能力…
            </span>
          )}
        </div>
        {notice && (
          <p className="import-formats__notice" role="status">
            {notice}
          </p>
        )}
        <ul className="import-formats__list" role="list" aria-busy={loading}>
          {formats.map(format => (
            <li
              key={format.format}
              className={`import-formats__item import-formats__item--${format.status}`}
              aria-label={`${format.label}：${STATUS_LABEL[format.status]}`}
            >
              <span className="import-formats__badge" aria-hidden="true">
                {STATUS_ICON[format.status]}
              </span>
              <span className="import-formats__name">{format.label}</span>
              <span className="import-formats__ext">
                {format.extensions.join(', ')}
              </span>
              <span className={`import-formats__tag import-formats__tag--${format.status}`}>
                {STATUS_LABEL[format.status]}
              </span>
              {format.note && (
                <span className="import-formats__note">{format.note}</span>
              )}
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}
