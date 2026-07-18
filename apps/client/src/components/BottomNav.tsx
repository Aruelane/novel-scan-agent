import type { MobilePanel } from '../domain';
import './BottomNav.css';

interface BottomNavProps {
  currentPanel: MobilePanel;
  onPanelChange: (panel: MobilePanel) => void;
}

interface NavItem {
  key: MobilePanel;
  label: string;
  /** SVG path data for a 24x24 icon */
  iconPath: string;
}

const NAV_ITEMS: NavItem[] = [
  {
    key: 'bookshelf',
    label: '书架',
    iconPath: 'M4 6h16M4 12h16M4 18h16',
  },
  {
    key: 'workspace',
    label: '工作区',
    iconPath: 'M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5',
  },
  {
    key: 'evidence',
    label: '命中',
    iconPath: 'M9 5H7a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h2M15 5h2a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2h-2M16 12H8',
  },
];

export function BottomNav({ currentPanel, onPanelChange }: BottomNavProps) {
  return (
    <nav className="bottom-nav" role="navigation" aria-label="移动端导航">
      {NAV_ITEMS.map(item => {
        const isActive = currentPanel === item.key;
        return (
          <button
            key={item.key}
            className={`bottom-nav__btn${isActive ? ' bottom-nav__btn--active' : ''}`}
            onClick={() => onPanelChange(item.key)}
            aria-current={isActive ? 'page' : undefined}
            aria-label={item.label}
          >
            <svg
              className="bottom-nav__icon"
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth={isActive ? 2.2 : 1.8}
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
            >
              <path d={item.iconPath} />
            </svg>
            <span className="bottom-nav__label">{item.label}</span>
          </button>
        );
      })}
    </nav>
  );
}
