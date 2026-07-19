import { invoke, isTauri } from '@tauri-apps/api/core';

export interface RuleSummaryDto {
  id: string;
  version: number;
  name: string;
  category: string;
  defaultSeverity: string;
  status: string;
  detectionMode: string;
  criteriaCount: number;
  exclusionsCount: number;
  pendingConditionsCount: number;
}

export interface RulePackSummaryDto {
  id: string;
  version: string;
  schemaVersion: string;
  ruleCount: number;
  rules: RuleSummaryDto[];
}

const STATIC_SUMMARY: RulePackSummaryDto = {
  id: 'community.yy-novel-bar',
  version: '2026.0.0-seed.1',
  schemaVersion: '1.0.0',
  ruleCount: 32,
  rules: [],
};

export async function loadRulePackSummary(): Promise<RulePackSummaryDto> {
  if (!isTauri()) {
    return { ...STATIC_SUMMARY };
  }
  try {
    return await invoke<RulePackSummaryDto>('rule_pack_summary');
  } catch {
    return { ...STATIC_SUMMARY };
  }
}
