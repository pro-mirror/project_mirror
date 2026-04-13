export const theme = {
  colors: {
    background: '#0F172A',       // Deep Navy
    backgroundAlt: '#1E293B',    // Blue Grey
    text: '#F5F5F4',             // Sand Beige
    textSecondary: '#94A3B8',    // Slate Grey
    accent: '#ee787cfd',           // Sky Blue (感謝)
    accentDanger: '#F87171',     // Red (負の感情)
    border: '#334155',           // Border color
  },
  spacing: {
    xs: 4,
    sm: 8,
    md: 16,
    lg: 24,
    xl: 32,
    xxl: 48,
  },
  borderRadius: {
    sm: 8,
    md: 16,
    lg: 24,
    full: 9999,
  },
  fontSize: {
    xs: 12,
    sm: 14,
    md: 16,
    lg: 18,
    xl: 24,
    xxl: 32,
  },
  fontFamily: {
    regular: 'System',
    medium: 'System',
    bold: 'System',
  },
};

export type Theme = typeof theme;
