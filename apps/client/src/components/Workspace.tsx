import { useRef, useCallback, type KeyboardEvent } from 'react';
import type {
  AppSettings,
  Book,
  FormatInfo,
  Rule,
  ScanJob,
  WorkspaceTab,
} from '../domain';
import { ImportPanel } from './ImportPanel';
import { RuleSelector } from './RuleSelector';
import { ScanProgress } from './ScanProgress';
import { SettingsPanel } from './SettingsPanel';
import './Workspace.css';

interface WorkspaceProps {
  books: Book[];
  rules: Rule[];
  jobs: ScanJob[];
  selectedBookId: string | null;
  activeTab: WorkspaceTab;
  settings: AppSettings;
  formatCapabilities: FormatInfo[];
  formatCapabilitiesLoading: boolean;
  formatCapabilitiesNotice: string | null;
  importError: string | null;
  onTabChange: (tab: WorkspaceTab) => void;
  onToggleRule: (ruleId: string) => void;
  onSetRuleSeverity: (ruleId: string, severity: 1 | 2 | 3 | 4 | 5) => void;
  onPauseScan: (jobId: string) => void;
  onResumeScan: (jobId: string) => void;
  onImportBook: (sourceName: string, bytes: Uint8Array) => Promise<string>;
  onClearImportError: () => void;
  className?: string;
}

const TABS: { key: WorkspaceTab; label: string }[] = [
  { key: 'import', label: '导入' },
  { key: 'rules', label: '规则' },
  { key: 'scan', label: '扫描' },
  { key: 'settings', label: '设置' },
];

export function Workspace({
  books,
  rules,
  jobs,
  selectedBookId,
  activeTab,
  settings,
  formatCapabilities,
  formatCapabilitiesLoading,
  formatCapabilitiesNotice,
  importError,
  onTabChange,
  onToggleRule,
  onSetRuleSeverity,
  onPauseScan,
  onResumeScan,
  onImportBook,
  onClearImportError,
  className,
}: WorkspaceProps) {
  const selectedBook = books.find(b => b.id === selectedBookId) ?? null;
  const activeJob = jobs.find(j => j.bookId === selectedBookId) ?? null;

  const tabRefs = useRef<(HTMLButtonElement | null)[]>([]);

  const activeIndex = TABS.findIndex(t => t.key === activeTab);

  const handleTabKeyDown = useCallback((event: KeyboardEvent<HTMLButtonElement>, index: number) => {
    let nextIndex: number;
    const len = TABS.length;
    switch (event.key) {
      case 'ArrowLeft':
        event.preventDefault();
        nextIndex = (index - 1 + len) % len;
        break;
      case 'ArrowRight':
        event.preventDefault();
        nextIndex = (index + 1) % len;
        break;
      case 'Home':
        event.preventDefault();
        nextIndex = 0;
        break;
      case 'End':
        event.preventDefault();
        nextIndex = len - 1;
        break;
      default:
        return;
    }
    tabRefs.current[nextIndex]?.focus();
    onTabChange(TABS[nextIndex].key);
  }, [onTabChange]);

  return (
    <main className={`workspace ${className ?? ''}`} role="main" aria-label="工作区">
      {/* 标签导航 */}
      <nav className="workspace-tabs" role="tablist" aria-label="工作区标签">
        {TABS.map((tab, index) => {
          const isActive = activeTab === tab.key;
          return (
            <button
              key={tab.key}
              id={`tab-${tab.key}`}
              ref={el => { tabRefs.current[index] = el; }}
              role="tab"
              className={`workspace-tab${isActive ? ' workspace-tab--active' : ''}`}
              aria-selected={isActive}
              aria-controls={`panel-${tab.key}`}
              tabIndex={isActive ? 0 : -1}
              onClick={() => onTabChange(tab.key)}
              onKeyDown={e => handleTabKeyDown(e, index)}
            >
              {tab.label}
            </button>
          );
        })}
      </nav>

      {/* 标签内容 */}
      {TABS.map(tab => (
        <div
          key={tab.key}
          id={`panel-${tab.key}`}
          role="tabpanel"
          aria-labelledby={`tab-${tab.key}`}
          className="workspace-content"
          hidden={activeTab !== tab.key}
        >
          {tab.key === 'import' && (
            <ImportPanel
              formats={formatCapabilities}
              loading={formatCapabilitiesLoading}
              notice={formatCapabilitiesNotice}
              importError={importError}
              onImport={onImportBook}
              onClearError={onClearImportError}
            />
          )}

          {tab.key === 'rules' && (
            <RuleSelector
              rules={rules}
              onToggle={onToggleRule}
              onSetSeverity={onSetRuleSeverity}
            />
          )}

          {tab.key === 'scan' && (
            <ScanProgress
              book={selectedBook}
              job={activeJob}
              onPause={onPauseScan}
              onResume={onResumeScan}
            />
          )}

          {tab.key === 'settings' && (
            <SettingsPanel settings={settings} />
          )}
        </div>
      ))}
    </main>
  );
}
