import { useAppState } from './hooks/useAppState';
import { Sidebar } from './components/Sidebar';
import { Workspace } from './components/Workspace';
import { EvidencePanel } from './components/EvidencePanel';
import { BottomNav } from './components/BottomNav';
import './App.css';

export default function App() {
  const state = useAppState();

  const mobileClass = (panel: string) =>
    state.mobilePanel === panel ? 'mobile-visible' : '';

  return (
    <>
      <div className="app-layout" aria-label="扫文助手">
        <Sidebar
          books={state.books}
          jobs={state.jobs}
          selectedBookId={state.selectedBookId}
          onSelectBook={state.selectBook}
          className={mobileClass('bookshelf')}
        />
        <Workspace
          books={state.books}
          rules={state.rules}
          jobs={state.jobs}
          selectedBookId={state.selectedBookId}
          activeTab={state.activeTab}
          settings={state.settings}
          formatCapabilities={state.formatCapabilities}
          formatCapabilitiesLoading={state.formatCapabilitiesLoading}
          formatCapabilitiesNotice={state.formatCapabilitiesNotice}
          importError={state.importError}
          onTabChange={state.setActiveTab}
          onToggleRule={state.toggleRule}
          onSetRuleSeverity={state.setRuleSeverity}
          onPauseScan={state.pauseScan}
          onResumeScan={state.resumeScan}
          onStartScan={state.startScan}
          scanError={state.scanError}
          onImportBook={state.importBook}
          onClearImportError={state.clearImportError}
          className={mobileClass('workspace')}
        />
        <EvidencePanel
          hits={state.hits}
          jobs={state.jobs}
          selectedBookId={state.selectedBookId}
          onUpdateReview={state.updateHitReview}
          className={mobileClass('evidence')}
        />
      </div>

      <BottomNav
        currentPanel={state.mobilePanel}
        onPanelChange={state.setMobilePanel}
      />
    </>
  );
}
