import { invoke, isTauri } from '@tauri-apps/api/core';
import type { ScanJob, Hit, FindingStatus, HitReviewStatus } from '../domain';

// ── DTO types from Rust scan_commands ─────────────────────────

interface ScanJobDto {
  taskId: string;
  documentId: string;
  status: string;
  chapterPosition: number;
  totalChapters: number;
  findingsCount: number;
  stopReason: string | null;
}

interface FindingDto {
  id: string;
  ruleId: string;
  category: string;
  alertLevel: string;
  confidenceBps: number;
  rationale: string;
  status: string;
  chapterId: string;
  chapterOrdinal: number;
  chapterTitle: string;
  evidenceCount: number;
}

interface EvidenceDetailDto {
  findingId: string;
  exactQuote: string;
  quoteHash: string;
  chapterId: string;
  chapterOrdinal: number;
  chapterTitle: string;
  utf8ByteStart: number;
  utf8ByteEnd: number;
}

// ── Mapping helpers ───────────────────────────────────────────

function mapStatus(dtoStatus: string): ScanJob['status'] {
  switch (dtoStatus) {
    case 'pending': return 'pending';
    case 'running': return 'running';
    case 'paused': return 'paused';
    case 'completed': return 'completed';
    case 'failed': return 'failed';
    default: return 'pending';
  }
}

function mapFindingStatus(dtoStatus: string): FindingStatus {
  switch (dtoStatus) {
    case 'suspected': return 'suspected';
    case 'pending_confirmation': return 'pending_confirmation';
    case 'confirmed': return 'confirmed';
    case 'rejected': return 'rejected';
    default: return 'suspected';
  }
}

function mapFindingToHit(f: FindingDto, jobId: string, taskId: string): Hit {
  return {
    id: f.id,
    jobId,
    ruleId: f.ruleId,
    ruleName: f.ruleId, // rule name comes from rule pack later
    chapter: f.chapterOrdinal + 1,
    chapterTitle: f.chapterTitle,
    position: `第 ${f.chapterOrdinal + 1} 章`,
    sourceKind: 'source_text',
    excerpt: '', // populated by evidence detail
    reason: f.rationale,
    confidence: f.alertLevel === 'critical' || f.alertLevel === 'high' ? 'high' : f.alertLevel === 'medium' ? 'medium' : 'low',
    findingStatus: mapFindingStatus(f.status),
    reviewStatus: f.status === 'confirmed' ? 'confirmed' : 'reviewing',
    foundAt: new Date().toISOString(),
  };
}

// ── Public API ─────────────────────────────────────────────────

export interface CreateScanJobResult {
  job: ScanJob;
}

export async function createScanJob(documentJson: string): Promise<CreateScanJobResult> {
  if (!isTauri()) throw new Error('扫描仅在桌面应用中可用');

  const dto = await invoke<ScanJobDto>('create_scan_job', { documentJson });

  const job: ScanJob = {
    id: dto.taskId,
    bookId: dto.documentId,
    ruleIds: [], // populated by the backend from seed rulepack
    status: mapStatus(dto.status),
    progress: dto.totalChapters > 0 ? dto.chapterPosition / dto.totalChapters : 0,
    currentChapter: dto.chapterPosition,
    totalChapters: dto.totalChapters,
    startedAt: new Date().toISOString(),
    compressionCount: 0,
    estimatedRemaining: dto.totalChapters - dto.chapterPosition,
  };

  return { job };
}

export async function runScanBatch(taskId: string, maxChapters?: number): Promise<ScanJob> {
  if (!isTauri()) throw new Error('扫描仅在桌面应用中可用');

  const dto = await invoke<ScanJobDto>('run_scan_batch', { taskId, maxChapters });

  return {
    id: dto.taskId,
    bookId: dto.documentId,
    ruleIds: [],
    status: mapStatus(dto.status),
    progress: dto.totalChapters > 0 ? dto.chapterPosition / dto.totalChapters : 0,
    currentChapter: dto.chapterPosition,
    totalChapters: dto.totalChapters,
    startedAt: new Date().toISOString(),
    compressionCount: 0,
    estimatedRemaining: dto.totalChapters - dto.chapterPosition,
  };
}

export async function getScanJob(taskId: string): Promise<ScanJob> {
  if (!isTauri()) throw new Error('扫描仅在桌面应用中可用');

  const dto = await invoke<ScanJobDto>('get_scan_job', { taskId });

  return {
    id: dto.taskId,
    bookId: dto.documentId,
    ruleIds: [],
    status: mapStatus(dto.status),
    progress: dto.totalChapters > 0 ? dto.chapterPosition / dto.totalChapters : 0,
    currentChapter: dto.chapterPosition,
    totalChapters: dto.totalChapters,
    startedAt: new Date().toISOString(),
    compressionCount: 0,
    estimatedRemaining: dto.totalChapters - dto.chapterPosition,
  };
}

export async function listFindings(taskId: string): Promise<Hit[]> {
  if (!isTauri()) throw new Error('扫描仅在桌面应用中可用');

  const dtos = await invoke<FindingDto[]>('list_findings', { taskId });

  return dtos.map(f => mapFindingToHit(f, taskId, taskId));
}
