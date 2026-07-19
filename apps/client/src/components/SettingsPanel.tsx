import type { AppSettings, ProviderConfig } from '../domain';
import './SettingsPanel.css';

interface SettingsPanelProps {
  settings: AppSettings;
}

/** 提供商对应的说明 */
function providerNote(type: string): string {
  switch (type) {
    case 'openai':    return '计划连接 OpenAI，或连接使用同类接口格式的服务。';
    case 'anthropic': return '计划连接 Anthropic 提供的 Claude 模型。';
    case 'gemini':    return '计划连接 Google 提供的 Gemini 模型。';
    case 'deepseek':  return '计划连接 DeepSeek 提供的在线模型。';
    case 'local':     return '为以后在自己的设备或局域网中运行模型预留。';
    default:          return '';
  }
}

export function SettingsPanel({ settings }: SettingsPanelProps) {
  return (
    <section className="settings-panel" aria-label="应用设置">
      <h3 className="settings-panel__title">选择帮你读书的模型</h3>
      <p className="settings-panel__desc">
        开发、浏览界面和运行本地测试都不需要 API Key。只有成品真正调用在线模型时，才需要配置相应服务的凭据。
      </p>

      {/* 提供商列表 */}
      <div className="settings-providers">
        {settings.providers.map((provider) => (
          <ProviderCard key={provider.type} provider={provider} />
        ))}
      </div>

      {/* 上下文设置 */}
      <h3 className="settings-panel__title" style={{ marginTop: 'var(--space-xl)' }}>长篇阅读记忆</h3>
      <div className="settings-context">
        <div className="settings-field">
          <label className="settings-field__label" htmlFor="context-window">
            单次阅读窗口（字符预算，演示值）
          </label>
          <input
            id="context-window"
            type="number"
            className="settings-field__input"
            value={settings.contextWindow}
            readOnly
            aria-describedby="context-window-hint"
            disabled
          />
          <span id="context-window-hint" className="settings-field__hint">
            较早章节会整理成结构化记忆；命中证据仍需回到原文位置核验。（演示版本不可修改）
          </span>
        </div>

        <div className="settings-check">
          <label className="settings-check__label">
            <input
              type="checkbox"
              checked={settings.autoCompress}
              readOnly
              disabled
            />
            <span>自动整理前文</span>
          </label>
          <span className="settings-check__hint">
            阅读窗口接近上限时，把人物关系、事件和待确认线索整理后继续向后读。
          </span>
        </div>
      </div>

      {/* 演示提示 */}
      <div className="settings-demo-hint">
        <span aria-hidden="true">[i]</span>
        <span>
          当前只展示配置状态，不会发送网络请求。后续由原生安全存储保管凭据，前端不会读取密钥明文。
        </span>
      </div>
    </section>
  );
}

function ProviderCard({ provider }: { provider: ProviderConfig }) {
  return (
    <div className={`provider-card${provider.enabled ? ' provider-card--enabled' : ''}`} aria-label={`${provider.label} 提供商`}>
      <div className="provider-card__header">
        <span className="provider-card__name">{provider.label}</span>
        <span className={`provider-card__status${provider.enabled ? ' provider-card__status--on' : ''}`}>
          {provider.enabled ? '已启用' : '未启用'}
        </span>
      </div>

      <p className="provider-card__note">{providerNote(provider.type)}</p>

      <div className="provider-card__fields">
        <div className="provider-field">
          <label className="provider-field__label">接口地址</label>
          <input
            type="text"
            className="provider-field__input"
            value={provider.endpoint}
            readOnly
            disabled
          />
        </div>
        <div className="provider-field">
          <label className="provider-field__label">模型</label>
          <input
            type="text"
            className="provider-field__input"
            value={provider.model || '尚未选择'}
            readOnly
            disabled
          />
        </div>
        <div className="provider-field">
          <label className="provider-field__label">访问凭据</label>
          <input
            type="text"
            className="provider-field__input"
            value={
              provider.credentialState === 'configured'
                ? '已安全保存'
                : provider.credentialState === 'unavailable'
                  ? '不可用'
                  : '尚未配置'
            }
            readOnly
            disabled
          />
        </div>
      </div>
    </div>
  );
}
