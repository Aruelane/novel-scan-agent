import { useState, useCallback, useEffect } from 'react';
import { isTauri } from '@tauri-apps/api/core';
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
import { importBookBytes } from '../services/importBooks';

/** Manages the full application state, with real data when Tauri is available. */
export function useAppState() {
  const isNative = isTauri();

  // Start empty in Tauri, use demos only in browser preview
  const [books, setBooks] = useState<Book[]>(isNative ? [] : demoBooks);
  const [rules, setRules] = useState<Rule[]>(demoRules);
  const [jobs, setJobs] = useState<ScanJob[]>(isNative ? [] : demoScanJobs);
  const [hits, setHits] = useState<Hit[]>(isNative ? [] : demoHits);
  const [settings, setSettings] = useState<AppSettings>(demoSettings);
  const [formatCapabilities, setFormatCapabilities] = useState<FormatInfo[]>(
    () => ALL_FORMATS.map(format => ({ ...format, extensions: [...format.extensions] })),
  );
  const [formatCapabilitiesLoading, setFormatCapabilitiesLoading] = useState(true);
  const [formatCapabilitiesNotice, setFormatCapabilitiesNotice] = useState<string | null>(null);
  const [importError, setImportError] = useState<string | null>(null);

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

  // ── UI state ──

  const [selectedBookId, setSelectedBookId] = useState<string | null>(
    isNative ? null : 'book-001',
  );
  const [activeTab, setActiveTab] = useState<WorkspaceTab>('scan');
  const [mobilePanel, setMobilePanel] = useState<MobilePanel>('workspace');

  // ── Operations ──

  /** Import a book via Tauri native dialog or direct file bytes. */
  const importBook = useCallback(async (sourceName: string, bytes: Uint8Array) => {
    setImportError(null);
    try {
      const result = await importBookBytes(sourceName, bytes);
      setBooks(prev => [result.book, ...prev]);
      if (!selectedBookId) {
        setSelectedBookId(result.book.id);
      }
      return result.summary;
    } catch (err) {
      const message = err instanceof Error ? err.message : '导入失败';
      setImportError(message);
      throw err;
    }
  }, [selectedBookId]);

  const clearImportError = useCallback(() => setImportError(null), []);

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
    importError,
    selectedBookId,
    activeTab,
    mobilePanel,
    setActiveTab,
    setMobilePanel,
    importBook,
    clearImportError,
    toggleRule,
    setRuleSeverity,
    pauseScan,
    resumeScan,
    updateHitReview,
    selectBook,
  };
}
