import { useState, useMemo } from 'react';
import type { Rule, RuleCategory, Severity } from '../domain';
import { categoryLabel, severityLabel } from '../domain';
import './RuleSelector.css';

const SEVERITIES: Severity[] = [1, 2, 3, 4, 5];

interface RuleSelectorProps {
  rules: Rule[];
  onToggle: (ruleId: string) => void;
  onSetSeverity: (ruleId: string, severity: Severity) => void;
}

export function RuleSelector({ rules, onToggle, onSetSeverity }: RuleSelectorProps) {
  const [expandedCategory, setExpandedCategory] = useState<RuleCategory | null>('landmine');

  const grouped = useMemo(() => {
    const map: Record<RuleCategory, Rule[]> = { landmine: [], frustration: [] };
    for (const r of rules) {
      map[r.category].push(r);
    }
    return map;
  }, [rules]);

  const toggleCategory = (cat: RuleCategory) => {
    setExpandedCategory(prev => prev === cat ? null : cat);
  };

  return (
    <section className="rule-selector" aria-label="扫描规则配置">
      <h3 className="rule-selector__title">扫描规则</h3>
      <p className="rule-selector__desc">
        选择你关心的雷点和郁闷点，并设置严重程度。扫描引擎接通后，会按这份选择逐章检查。
      </p>
      <p className="rule-selector__disclaimer">
        当前为界面演示规则；正式社区规则包以版本和核验状态为准，未核验条目默认关闭。
      </p>

      {/* 分类 */}
      {(['landmine', 'frustration'] as RuleCategory[]).map(cat => {
        const catRules = grouped[cat];
        const enabledCount = catRules.filter(r => r.enabled).length;
        const isExpanded = expandedCategory === cat;

        return (
          <div key={cat} className={`rule-category${cat === 'landmine' ? ' rule-category--landmine' : ''}`}>
            <button
              className="rule-category__header"
              onClick={() => toggleCategory(cat)}
              aria-expanded={isExpanded}
              aria-label={`${categoryLabel(cat)}，${enabledCount}/${catRules.length} 条已启用`}
            >
              <span className="rule-category__icon" aria-hidden="true">
                {isExpanded ? '[-]' : '[+]'}
              </span>
              <span className="rule-category__name">{categoryLabel(cat)}</span>
              <span className="rule-category__count">
                {enabledCount}/{catRules.length}
              </span>
            </button>

            {isExpanded && (
              <ul className="rule-category__list" role="list">
                {catRules.map(rule => (
                  <li key={rule.id} className={`rule-item${rule.enabled ? '' : ' rule-item--disabled'}`}>
                    <div className="rule-item__header">
                      <label className="rule-item__toggle" aria-label={`切换规则「${rule.name}」`}>
                        <input
                          type="checkbox"
                          checked={rule.enabled}
                          onChange={() => onToggle(rule.id)}
                          className="rule-item__checkbox"
                        />
                        <span className="rule-item__switch" aria-hidden="true" />
                        <span className="rule-item__name">{rule.name}</span>
                      </label>

                      <div className="rule-item__severity" role="radiogroup" aria-label={`${rule.name} 严重程度`}>
                        {SEVERITIES.map((s, idx) => (
                          <button
                            key={s}
                            className={`severity-btn severity-btn--${s}${rule.severity === s ? ' severity-btn--active' : ''}`}
                            role="radio"
                            aria-checked={rule.severity === s}
                            aria-label={`${severityLabel(s)}`}
                            tabIndex={rule.severity === s ? 0 : -1}
                            onClick={() => onSetSeverity(rule.id, s)}
                            onKeyDown={(event) => {
                              let next: number;
                              const cur = SEVERITIES.indexOf(rule.severity);
                              switch (event.key) {
                                case 'ArrowRight': case 'ArrowDown':
                                  event.preventDefault();
                                  next = (cur + 1) % SEVERITIES.length;
                                  onSetSeverity(rule.id, SEVERITIES[next]);
                                  break;
                                case 'ArrowLeft': case 'ArrowUp':
                                  event.preventDefault();
                                  next = (cur - 1 + SEVERITIES.length) % SEVERITIES.length;
                                  onSetSeverity(rule.id, SEVERITIES[next]);
                                  break;
                                case 'Home':
                                  event.preventDefault();
                                  onSetSeverity(rule.id, SEVERITIES[0]);
                                  break;
                                case 'End':
                                  event.preventDefault();
                                  onSetSeverity(rule.id, SEVERITIES[SEVERITIES.length - 1]);
                                  break;
                              }
                            }}
                          >
                            {s}
                          </button>
                        ))}
                      </div>
                    </div>
                    <p className="rule-item__desc">{rule.description}</p>
                    {rule.keywords.length > 0 && (
                      <p className="rule-item__keywords">
                        {rule.keywords.map(k => (
                          <span key={k} className="rule-item__keyword">{k}</span>
                        ))}
                      </p>
                    )}
                  </li>
                ))}
              </ul>
            )}
          </div>
        );
      })}
    </section>
  );
}
