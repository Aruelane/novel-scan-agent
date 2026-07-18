import { useState, useCallback, useEffect } from 'react';
import type {
  Book,
  Rule,
  ScanJob,
  Hit,
  AppSettings,
  FormatInfo,
  WorkspaceTab,
  MobilePanel,
} from '../domain';
import { ALL_FORMATS } from '../domain';
import {
  demoBooks,
  demoRules,
  demoScanJobs,
  demoHits,
  demoSettings,
} from '../demo-data';
import { loadImportCapabilities } from '../services/importCapabilities';

/** 管理整个前端演示状态的 hook */
export function useAppState() {
  const [books, setBooks] = useState<Book[]>(demoBooks);
  const [rules, setRules] = useState<Rule[]>(demoRules);
  const [jobs, setJobs] = useState<ScanJob[]>(demoScanJobs);
  const [hits, setHits] = useState<Hit[]>(demoHits);
  const [settings, setSettings] = useState<AppSettings>(demoSettings);
  const [formatCapabilities, setFormatCapabilities] = useState<FormatInfo[]>(
    () => ALL_FORMATS.map(format => ({ ...format, extensions: [...format.extensions] })),
  );
  const [formatCapabilitiesLoading, setFormatCapabilitiesLoading] = useState(true);
  const [formatCapabilitiesNotice, setFormatCapabilitiesNotice] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    void loadImportCapabilities().then(result => {
      if (cancelled) return;
      setFormatCapabilities(result.formats);
      setFormatCapabilitiesNotice(result.notice);
      setFormatCapabilitiesLoading(false);
    });

    return () => {
      cancelled = true;
    };
  }, []);

  // 桌面端：当前选中的书籍 ID
  const [selectedBookId, setSelectedBookId] = useState<string | null>('book-001');
  // 桌面端：工作区当前标签页
  const [activeTab, setActiveTab] = useState<WorkspaceTab>('scan');

  // 移动端：当前面板
  const [mobilePanel, setMobilePanel] = useState<MobilePanel>('workspace');

  // ── 操作 ──

  const toggleRule = useCallback((ruleId: string) => {
    setRules(prev => prev.map(r =>
      r.id === ruleId ? { ...r, enabled: !r.enabled } : r
    ));
  }, []);

  const setRuleSeverity = useCallback((ruleId: string, severity: 1 | 2 | 3 | 4 | 5) => {
    setRules(prev => prev.map(r =>
      r.id === ruleId ? { ...r, severity } : r
    ));
  }, []);

  const pauseScan = useCallback((jobId: string) => {
    setJobs(prev => prev.map(j =>
      j.id === jobId ? { ...j, status: 'paused' as const } : j
    ));
  }, []);

  const resumeScan = useCallback((jobId: string) => {
    setJobs(prev => prev.map(j =>
      j.id === jobId ? { ...j, status: 'running' as const } : j
    ));
  }, []);

  const updateHitReview = useCallback((hitId: string, status: 'confirmed' | 'false_positive') => {
    setHits(prev => prev.map(h =>
      h.id === hitId ? { ...h, reviewStatus: status } : h
    ));
  }, []);

  const selectBook = useCallback((bookId: string) => {
    setSelectedBookId(bookId);
    setActiveTab('scan');
  }, []);

  return {
    books,
    rules,
    jobs,
    hits,
    settings,
    formatCapabilities,
    formatCapabilitiesLoading,
    formatCapabilitiesNotice,
    selectedBookId,
    activeTab,
    mobilePanel,
    setActiveTab,
    setMobilePanel,
    toggleRule,
    setRuleSeverity,
    pauseScan,
    resumeScan,
    updateHitReview,
    selectBook,
  };
}
