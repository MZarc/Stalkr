// Deterministic, offline-safe luxury avatar generator
// Never breaks, requires 0 network requests, immune to CORS and 403 Forbidden.

const LUXURY_GRADIENTS = [
  ['#7c3aed', '#a855f7'], // Violet
  ['#0284c7', '#38bdf8'], // Cyan
  ['#059669', '#34d399'], // Emerald
  ['#e11d48', '#fb7185'], // Rose
  ['#d97706', '#fbbf24'], // Amber
  ['#4f46e5', '#818cf8'], // Indigo
  ['#c026d3', '#e879f9'], // Fuchsia
  ['#ea580c', '#fb923c'], // Coral
  ['#0d9488', '#2dd4bf'], // Teal
  ['#9333ea', '#c084fc'], // Purple
  ['#dc2626', '#f87171'], // Crimson
  ['#2563eb', '#60a5fa'], // Sapphire
];

export function getFallbackAvatar(username: string): string {
  const clean = (username || 'user').replace(/[^a-zA-Z0-9]/g, '');
  const initials = clean.substring(0, 2).toUpperCase() || 'IG';

  let hash = 0;
  for (let i = 0; i < username.length; i++) {
    hash = (hash << 5) - hash + username.charCodeAt(i);
    hash |= 0;
  }
  const pair = LUXURY_GRADIENTS[Math.abs(hash) % LUXURY_GRADIENTS.length];

  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <defs>
      <linearGradient id="g" x1="0%" y1="0%" x2="100%" y2="100%">
        <stop offset="0%" stop-color="${pair[0]}"/>
        <stop offset="100%" stop-color="${pair[1]}"/>
      </linearGradient>
    </defs>
    <rect width="100" height="100" rx="50" fill="url(#g)"/>
    <circle cx="50" cy="50" r="48" fill="none" stroke="rgba(255,255,255,0.2)" stroke-width="2"/>
    <text x="50" y="52" font-family="-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif" font-size="36" font-weight="700" fill="#ffffff" text-anchor="middle" dominant-baseline="central">${initials}</text>
  </svg>`;

  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

export function getAvatarUrl(username: string, rawUrl?: string | null): string {
  // If the URL is empty or is an external Unsplash link that frequently 403s in webviews
  if (!rawUrl || rawUrl.trim().length === 0 || rawUrl.includes('images.unsplash.com')) {
    return getFallbackAvatar(username);
  }
  return rawUrl;
}
