/* ============================================================
 * demo-data.ts — 演示用静态数据
 * 后续替换为 Tauri command / Rust 后端返回的真实数据。
 * ============================================================ */

import type {
  Book,
  Rule,
  ScanJob,
  Hit,
  AppSettings,
  ProviderConfig,
} from './domain';

// ── 演示书籍 ──

export const demoBooks: Book[] = [
  {
    id: 'book-001',
    title: '扫评演示·潮汐信标 A01',
    author: '界面演示文本, 非真实出版物',
    format: 'txt',
    sourceDisplayName: '扫评演示·潮汐信标 A01',
    sourceRef: '/demo/original/扫评演示·潮汐信标_A01.txt',
    status: 'scanned',
    addedAt: '2026-07-15T10:30:00Z',
    totalChapters: 380,
    fileSize: 5_200_000,
  },
  {
    id: 'book-002',
    title: '扫评演示·灰烬航标 B02',
    author: '界面演示文本, 非真实出版物',
    format: 'txt',
    sourceDisplayName: '扫评演示·灰烬航标 B02',
    sourceRef: '/demo/original/扫评演示·灰烬航标_B02.txt',
    status: 'idle',
    addedAt: '2026-07-16T08:00:00Z',
    totalChapters: 1648,
    fileSize: 12_800_000,
  },
  {
    id: 'book-003',
    title: '扫评演示·纸月迷廊 C03',
    author: '界面演示文本, 非真实出版物',
    format: 'markdown',
    sourceDisplayName: '扫评演示·纸月迷廊 C03',
    sourceRef: '/demo/original/扫评演示·纸月迷廊_C03.md',
    status: 'idle',
    addedAt: '2026-07-17T14:20:00Z',
    totalChapters: 1430,
    fileSize: 9_100_000,
  },
];

// ── 演示规则 ──

export const demoRules: Rule[] = [
  // 雷点
  {
    id: 'rule-001',
    name: '绿帽 / NTR',
    category: 'landmine',
    severity: 5,
    enabled: true,
    description: '主角伴侣被他人夺走、感情背叛、或存在明确的 NTR 情节',
    keywords: ['绿帽', 'NTR', '出轨', '背叛', '夺走', '被抢走'],
  },
  {
    id: 'rule-002',
    name: '虐主 / 憋屈',
    category: 'landmine',
    severity: 4,
    enabled: true,
    description: '主角长期被压制、羞辱而无合理反击，或结局悲惨',
    keywords: ['虐主', '憋屈', '羞辱', '无力反抗', '悲惨'],
  },
  {
    id: 'rule-003',
    name: '送女 / 漏女',
    category: 'landmine',
    severity: 5,
    enabled: false,
    description: '与主角有情感羁绊的女性角色被送给他人或莫名消失',
    keywords: ['送女', '漏女', '让给', '送给', '消失'],
  },
  {
    id: 'rule-004',
    name: '太监 / 烂尾',
    category: 'landmine',
    severity: 3,
    enabled: true,
    description: '作品未完结且长期断更，或结局明显仓促敷衍',
    keywords: ['太监', '烂尾', '断更', '草草收场'],
  },
  {
    id: 'rule-005',
    name: '伴侣受侵害 / 文青虐心',
    category: 'landmine',
    severity: 4,
    enabled: false,
    description: '主角伴侣遭受侵害，或叙事刻意以关系伤害制造虐心感',
    keywords: ['伴侣受侵害', '文青虐心', '侵犯', '关系伤害'],
  },
  {
    id: 'rule-006',
    name: '死女 / 死重要角色',
    category: 'landmine',
    severity: 4,
    enabled: true,
    description: '重要女性角色或战友死亡且非剧情必须',
    keywords: ['死了', '去世', '牺牲', '陨落'],
  },

  // 郁闷点
  {
    id: 'rule-007',
    name: '主角智商掉线',
    category: 'frustration',
    severity: 2,
    enabled: true,
    description: '主角做出明显不合逻辑的降智行为推动剧情',
    keywords: ['降智', '智商下线', '不合理', '强行'],
  },
  {
    id: 'rule-008',
    name: '剧情拖沓 / 水字数',
    category: 'frustration',
    severity: 1,
    enabled: false,
    description: '大量重复描写、无意义对话、明显水字数',
    keywords: ['水字数', '拖沓', '重复', '啰嗦'],
  },
  {
    id: 'rule-009',
    name: '三观不正',
    category: 'frustration',
    severity: 3,
    enabled: true,
    description: '作品传递明显有害的价值观念',
    keywords: ['三观', '不正', '扭曲'],
  },
  {
    id: 'rule-010',
    name: '系统 / 金手指过强',
    category: 'frustration',
    severity: 2,
    enabled: false,
    description: '主角开局获得过强系统或外挂，失去成长感',
    keywords: ['系统', '金手指', '外挂', '开局无敌'],
  },
  {
    id: 'rule-011',
    name: '女角色脸谱化',
    category: 'frustration',
    severity: 2,
    enabled: true,
    description: '女性角色刻板单一，缺乏独立人格',
    keywords: ['脸谱化', '花瓶', '刻板', '单一'],
  },
  {
    id: 'rule-012',
    name: '反派强行洗白',
    category: 'frustration',
    severity: 2,
    enabled: false,
    description: '作恶多端的反派被强行洗白加入主角阵营',
    keywords: ['洗白', '原谅', '强行和解'],
  },
];

// ── 演示扫描作业 ──

export const demoScanJobs: ScanJob[] = [
  {
    id: 'job-001',
    bookId: 'book-001',
    ruleIds: ['rule-001', 'rule-002', 'rule-004', 'rule-007', 'rule-009'],
    status: 'running',
    progress: 0.64,
    currentChapter: 243,
    totalChapters: 380,
    startedAt: '2026-07-17T16:00:00Z',
    compressionCount: 2,
    lastCompressionAt: '2026-07-17T16:12:30Z',
    estimatedRemaining: 185,
  },
];

// ── 演示命中结果 ──

export const demoHits: Hit[] = [
  {
    id: 'hit-001',
    jobId: 'job-001',
    ruleId: 'rule-002',
    ruleName: '虐主 / 憋屈',
    chapter: 87,
    chapterTitle: '无人回信',
    position: '第 34 段（约 62%）',
    sourceKind: 'original_demo',
    excerpt:
      '雨从仓库破窗斜落进来。闻舟被两名守卫按住肩膀，膝盖重重磕在湿冷的砖面上。顾临川把那封求援信扔进水洼，慢慢说道：”你的人已经走了，今晚不会有人来。”闻舟盯着纸页化开，始终没有等到同伴回应。',
    reason:
      '本章连续描写主角受制、求援落空且没有形成有效反击，符合当前启用的”虐主 / 憋屈”规则。这是界面演示证据，尚未通过核心从真实原文重建。',
    confidence: 'high',
    findingStatus: 'confirmed',
    reviewStatus: 'confirmed',
    foundAt: '2026-07-17T16:03:15Z',
  },
  {
    id: 'hit-002',
    jobId: 'job-001',
    ruleId: 'rule-007',
    ruleName: '主角智商掉线',
    chapter: 156,
    chapterTitle: '最后一盏灯',
    position: '第 12 段（约 28%）',
    sourceKind: 'original_demo',
    excerpt:
      '渡船只剩最后一盏灯。明知引路人前两次都给了假坐标，闻舟还是把避开巡逻的路线画在桌上，连备用码头也一并圈出。同行人问是否要留后手，他摇头说：”这次我愿意信他。”',
    reason:
      '这个决定与前文建立的谨慎性格存在冲突，但也可能是尚未揭晓的诱敌安排。因此先保留为疑似线索，等扫描后续章节再确认。这是界面演示证据，尚未通过核心从真实原文重建。',
    confidence: 'medium',
    findingStatus: 'pending_confirmation',
    reviewStatus: 'reviewing',
    foundAt: '2026-07-17T16:08:42Z',
  },
];

// ── 演示设置 ──

export const demoProviders: ProviderConfig[] = [
  {
    type: 'openai',
    label: 'OpenAI 兼容',
    credentialState: 'missing',
    endpoint: 'https://api.openai.com/v1',
    model: '',
    enabled: false,
  },
  {
    type: 'anthropic',
    label: 'Anthropic Claude',
    credentialState: 'missing',
    endpoint: 'https://api.anthropic.com',
    model: '',
    enabled: false,
  },
  {
    type: 'gemini',
    label: 'Google Gemini',
    credentialState: 'missing',
    endpoint: 'https://generativelanguage.googleapis.com',
    model: '',
    enabled: false,
  },
  {
    type: 'deepseek',
    label: 'DeepSeek',
    credentialState: 'missing',
    endpoint: 'https://api.deepseek.com',
    model: '',
    enabled: false,
  },
  {
    type: 'local',
    label: '本地模型',
    credentialState: 'missing',
    endpoint: 'http://localhost:11434/v1',
    model: '',
    enabled: false,
  },
];

export const demoSettings: AppSettings = {
  providers: demoProviders,
  contextWindow: 128_000,
  autoCompress: true,
  compressThreshold: 0.05,
};
