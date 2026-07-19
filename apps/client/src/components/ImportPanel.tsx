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
}

export function ImportPanel({ formats, loading, notice }: ImportPanelProps) {
  return (
    <section className="import-panel" aria-label="导入小说文件">
      <h3 className="import-panel__title">把小说交给我</h3>
      <p className="import-panel__desc">
        不限于 TXT。这里会逐步接收电子书、文档和压缩包，并如实告诉你哪些格式现在能读。
      </p>

      <div
        className="import-dropzone"
        aria-label="文件选择区域，当前为界面演示模式，暂未连接桌面或 Android 外壳"
        aria-disabled="true"
        onDragOver={(event) => { event.preventDefault(); }}
        onDrop={(event) => { event.preventDefault(); }}
      >
        <div className="import-dropzone__icon" aria-hidden="true">
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="17 8 12 3 7 8" />
            <line x1="12" y1="3" x2="12" y2="15" />
          </svg>
        </div>
        <p className="import-dropzone__text">
          导入功能将在 S2 多格式解析完成后接入
        </p>
        <p className="import-dropzone__hint">
          当前支持 TXT 与 Markdown 的本地导入能力已进入核心，桌面与 Android 外壳的导入命令将在后续版本连接。
        </p>
      </div>

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
