/* ============================================================
 * domain.ts — 纯类型定义与工具函数
 * 后续对接 Tauri/Rust 时替换数据源即可，类型保持不变。
 * ============================================================ */

// ── 书籍 ──

/** 支持的文件格式 */
export type BookFormat =
  | 'epub'
  | 'pdf'
  | 'doc'
  | 'docx'
  | 'txt'
  | 'markdown'
  | 'html'
  | 'mobi'
  | 'azw3'
  | 'zip'
  | '7z';

/**
 * 格式能力状态，与 Rust 导入层的 Ready / Pending / Unsupported 一一对应。
 * `pending` 只代表能够识别文件类型，不能假装已经可以读取正文。
 */
export type FormatStatus = 'ready' | 'pending' | 'unsupported';

/** 文件格式元信息 */
export interface FormatInfo {
  format: BookFormat;
  label: string;
  extensions: string[];
  status: FormatStatus;
  note?: string;
}

/** 所有支持格式的列表 */
export const ALL_FORMATS: FormatInfo[] = [
  { format: 'txt',       label: 'TXT 纯文本',        extensions: ['.txt'],                        status: 'ready', note: '可读取 UTF-8 与带 BOM 的 UTF-16；GBK/GB18030 暂会给出明确提示' },
  { format: 'markdown',  label: 'Markdown',          extensions: ['.md', '.markdown', '.mdown', '.mkd'], status: 'ready', note: '可按标题或常见章节名切分，并保留原文行号' },
  { format: 'epub',      label: 'EPUB 电子书',       extensions: ['.epub'],                       status: 'pending', note: '目录、spine 与 CFI 锚点接入中' },
  { format: 'pdf',       label: 'PDF 文档',          extensions: ['.pdf'],                        status: 'pending', note: '文本层优先；扫描版将单独标注 OCR' },
  { format: 'docx',      label: 'Word 文档',         extensions: ['.docx'],                       status: 'pending', note: '标题层级、段落与脚注解析接入中' },
  { format: 'doc',       label: '旧版 Word 文档',    extensions: ['.doc'],                        status: 'unsupported', note: '请另存为 DOCX、TXT 或 PDF' },
  { format: 'html',      label: 'HTML 网页',          extensions: ['.html', '.htm', '.xhtml'],      status: 'pending', note: '正文抽取、标题层级与页面锚点接入中' },
  { format: 'mobi',      label: 'MOBI 电子书',        extensions: ['.mobi', '.prc'],                status: 'pending', note: '正文与目录解析接入中' },
  { format: 'azw3',      label: 'AZW3 (Kindle)',     extensions: ['.azw3', '.azw'],                status: 'pending', note: '仅计划支持无 DRM 文件' },
  { format: 'zip',       label: 'ZIP 压缩包',         extensions: ['.zip'],                        status: 'pending', note: '后续会安全解压并识别包内内容' },
  { format: '7z',        label: '7Z 压缩包',          extensions: ['.7z'],                         status: 'pending', note: '后续会安全解压并识别包内内容' },
];

/** 书籍状态 */
export type BookStatus = 'idle' | 'scanning' | 'scanned' | 'failed';

/** 一本书 */
export interface Book {
  id: string;
  title: string;
  author: string;
  format: BookFormat;
  /** User-facing name of the source, used as the display label. */
  sourceDisplayName: string;
  status: BookStatus;
  addedAt: string; // ISO 8601
  totalChapters: number;
  /** 文件大小，字节 */
  fileSize: number;
}

// ── 规则 ──

/** 规则分类，与 Rust RuleCategory 和数据库 effective_category 的 landmine/frustration 一致 */
export type RuleCategory = 'landmine' | 'frustration';

/** 严重程度 1-5 */
export type Severity = 1 | 2 | 3 | 4 | 5;

/** 一条扫描规则 */
export interface Rule {
  id: string;
  name: string;
  category: RuleCategory;
  severity: Severity;
  enabled: boolean;
  description: string;
  /** 触发关键词示例 */
  keywords: string[];
}

// ── 扫描作业 ──

/** 扫描状态 */
export type ScanStatus = 'pending' | 'running' | 'paused' | 'completed' | 'failed';

/** 一次扫描作业 */
export interface ScanJob {
  id: string;
  bookId: string;
  ruleIds: string[];
  status: ScanStatus;
  progress: number; // 0-1
  /** 当前处理章节 */
  currentChapter: number;
  totalChapters: number;
  startedAt: string;
  completedAt?: string;
  /** 最近一次上下文压缩时间 */
  lastCompressionAt?: string;
  /** 上下文压缩次数 */
  compressionCount: number;
  /** 预估剩余时间，秒 */
  estimatedRemaining: number;
}

// ── 命中 ──

/** 命中置信度 */
export type Confidence = 'high' | 'medium' | 'low';

/** 命中审核状态 */
export type HitReviewStatus = 'reviewing' | 'confirmed' | 'false_positive';

/** 证据链状态；与模型置信度、用户审核结果相互独立。 */
export type FindingStatus = 'suspected' | 'pending_confirmation' | 'confirmed' | 'rejected';

/** 一个扫雷命中结果 */
export interface Hit {
  id: string;
  jobId: string;
  ruleId: string;
  ruleName: string;
  chapter: number;
  chapterTitle: string;
  /** 章节内位置（如段落号或百分比） */
  position: string;
  /** 区分真实导入原文与项目自写的界面演示文本，避免误当作真实书摘。 */
  sourceKind: 'source_text' | 'original_demo';
  /** 原文摘录 */
  excerpt: string;
  /** AI 给出的理由 */
  reason: string;
  confidence: Confidence;
  findingStatus: FindingStatus;
  reviewStatus: HitReviewStatus;
  /** 发现时间 */
  foundAt: string;
}

// ── 设置 ──

/** LLM 提供商 */
export type ProviderType = 'openai' | 'anthropic' | 'gemini' | 'deepseek' | 'local';

export interface ProviderConfig {
  type: ProviderType;
  label: string;
  /** 前端只知道凭据是否已由原生安全存储保存，从不接收密钥明文。 */
  credentialState: 'missing' | 'configured' | 'unavailable';
  endpoint: string;
  model: string;
  enabled: boolean;
}

/** 应用全局设置 */
export interface AppSettings {
  providers: ProviderConfig[];
  /** 每次扫描的上下文窗口大小（token 数） */
  contextWindow: number;
  /** 是否自动压缩上下文 */
  autoCompress: boolean;
  /** 压缩触发阈值（进度百分比变化后触发） */
  compressThreshold: number;
}

// ── UI 状态 ──

/** 移动端当前面板 */
export type MobilePanel = 'bookshelf' | 'workspace' | 'evidence';

/** 主工作区选中的标签页 */
export type WorkspaceTab = 'import' | 'rules' | 'scan' | 'settings';

// ── 工具函数 ──

export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function formatDuration(seconds: number): string {
  const roundedSeconds = Number.isFinite(seconds)
    ? Math.max(0, Math.ceil(seconds))
    : 0;

  if (roundedSeconds < 60) return `${roundedSeconds} 秒`;
  if (roundedSeconds < 3600) {
    return `${Math.floor(roundedSeconds / 60)} 分 ${roundedSeconds % 60} 秒`;
  }

  // 小时级预估不展示秒数，因此向上取整到分钟，避免低估剩余时间。
  const roundedMinutes = Math.ceil(roundedSeconds / 60);
  return `${Math.floor(roundedMinutes / 60)} 时 ${roundedMinutes % 60} 分`;
}

export function bookStatusLabel(s: BookStatus): string {
  switch (s) {
    case 'idle': return '待扫描';
    case 'scanning': return '扫描中';
    case 'scanned': return '已扫描';
    case 'failed': return '失败';
  }
}

export function confidenceLabel(c: Confidence): string {
  switch (c) {
    case 'high': return '高置信';
    case 'medium': return '中置信';
    case 'low': return '低置信';
  }
}

export function findingStatusLabel(s: FindingStatus): string {
  switch (s) {
    case 'suspected': return '疑似线索';
    case 'pending_confirmation': return '待后文确认';
    case 'confirmed': return '原文已核验';
    case 'rejected': return '已排除';
  }
}

export function severityLabel(s: Severity): string {
  switch (s) {
    case 1: return '轻微';
    case 2: return '较低';
    case 3: return '中等';
    case 4: return '较重';
    case 5: return '严重';
  }
}

export function scanStatusLabel(s: ScanStatus): string {
  switch (s) {
    case 'pending': return '等待中';
    case 'running': return '扫描中';
    case 'paused': return '已暂停';
    case 'completed': return '已完成';
    case 'failed': return '失败';
  }
}

export function categoryLabel(c: RuleCategory): string {
  return c === 'landmine' ? '雷点' : '郁闷点';
}
