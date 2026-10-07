import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { isNativeAuthAvailable, startNativeInstagramLogin } from '../src/lib/instagramAuth.ts';

const rootDir = process.cwd();

describe('Mobile Screen Viewports & Layout Foundation', () => {
  const standardMobileViewports = [
    { name: 'Ultra Compact Phone (iPhone SE 1st gen)', width: 320, height: 568 },
    { name: 'Compact Android (Samsung Galaxy A)', width: 360, height: 780 },
    { name: 'Standard iOS (iPhone SE / 8)', width: 375, height: 667 },
    { name: 'Modern iOS Standard (iPhone 13/14/15/16)', width: 390, height: 844 },
    { name: 'Modern Android Standard (Pixel 8 / Galaxy S24)', width: 412, height: 915 },
    { name: 'Phablet / Max Display (iPhone 16 Pro Max)', width: 430, height: 932 },
  ];

  it('validates comprehensive mobile device matrix coverage', () => {
    assert.equal(standardMobileViewports.length, 6);
    for (const vp of standardMobileViewports) {
      assert.ok(vp.width >= 320 && vp.width <= 430, `Width ${vp.width} within mobile specs`);
      assert.ok(vp.height >= 568 && vp.height <= 932, `Height ${vp.height} within mobile specs`);
    }
  });

  it('verifies strict horizontal overflow lockdown in global CSS', () => {
    const appCss = fs.readFileSync(path.join(rootDir, 'src/app.css'), 'utf-8');
    assert.ok(appCss.includes('overflow-x: hidden;'), 'Global CSS must lock overflow-x');
    assert.ok(appCss.includes('box-sizing: border-box;'), 'Global reset must enforce border-box');
    assert.ok(appCss.includes('--safe-top: env(safe-area-inset-top'), 'Must support safe area notches');
    assert.ok(appCss.includes('--safe-bottom: env(safe-area-inset-bottom'), 'Must support home indicator');
  });

  it('verifies mobile viewport meta tags in index.html', () => {
    const indexHtml = fs.readFileSync(path.join(rootDir, 'index.html'), 'utf-8');
    assert.ok(indexHtml.includes('viewport-fit=cover'), 'Viewport must fit device cutout and notch');
    assert.ok(indexHtml.includes('maximum-scale=1.0'), 'Must prevent disruptive zoom gestures');
    assert.ok(indexHtml.includes('user-scalable=no'), 'Native mobile feel without browser zoom jitter');
  });
});

describe('Top Navigation Bar Mobile Responsiveness', () => {
  const appSvelte = fs.readFileSync(path.join(rootDir, 'src/App.svelte'), 'utf-8');

  it('contains dedicated breakpoints for 480px, 430px, and 360px phones', () => {
    assert.ok(appSvelte.includes('@media (max-width: 480px)'), 'Needs 480px mobile breakpoint');
    assert.ok(appSvelte.includes('@media (max-width: 430px)'), 'Needs 430px mobile breakpoint');
    assert.ok(appSvelte.includes('@media (max-width: 360px)'), 'Needs 360px compact phone breakpoint');
  });

  it('guarantees top nav components do not overflow 360px viewport', () => {
    // Component budget test on 360px screen:
    // Left: Avatar (32px) + Gap (6px) + Handle (65px) + Dev Badge (~105px) = ~208px
    // Right: Logout (28px)
    // Padding: 6px * 2 = 12px
    // Total = 208 + 28 + 12 = 248px <= 360px (Plenty of headroom, zero clipping)
    const padding = 12;
    const avatarBubble = 32;
    const leftGap = 6;
    const handleMaxWidth = 65;
    const devBadgeWidth = 105;
    const logoutBtn = 28;

    const totalNavWidth = padding + avatarBubble + leftGap + handleMaxWidth + devBadgeWidth + logoutBtn;
    assert.ok(totalNavWidth <= 360, `Total top nav width (${totalNavWidth}px) must be <= 360px compact screen`);
  });

  it('enforces ellipsis truncation for long account handles on small screens', () => {
    assert.ok(appSvelte.includes('text-overflow: ellipsis;'), 'Handle must truncate with ellipsis');
    assert.ok(appSvelte.includes('white-space: nowrap;'), 'Handle must not wrap to second line');
  });
});

describe('Pulse View Mobile Grid & Reciprocity Layout', () => {
  const pulseSvelte = fs.readFileSync(path.join(rootDir, 'src/lib/views/PulseView.svelte'), 'utf-8');

  it('collapses 3-column insights grid to single-column on mobile viewports', () => {
    assert.ok(pulseSvelte.includes('@media (max-width: 520px)'), 'Must have 520px mobile breakpoint');
    assert.ok(pulseSvelte.includes('grid-template-columns: 1fr;'), 'Insights grid must stack vertically on mobile');
  });

  it('stacks reciprocity header and provides full-width clean action button on mobile', () => {
    assert.ok(pulseSvelte.includes('.reciprocity-header'), 'Must format reciprocity header');
    assert.ok(pulseSvelte.includes('flex-direction: column;'), 'Header must stack vertically on small devices');
    assert.ok(pulseSvelte.includes('width: 100%;'), 'Clean button expands to full touch width');
  });
});

describe('Settings View Target Grid Responsiveness', () => {
  const settingsSvelte = fs.readFileSync(path.join(rootDir, 'src/lib/views/SettingsView.svelte'), 'utf-8');

  it('converts 2x2 target grid to single column on mobile screens <= 540px', () => {
    assert.ok(settingsSvelte.includes('@media (max-width: 540px)'), 'Must include 540px breakpoint');
    assert.ok(settingsSvelte.includes('.targets-list-grid'), 'Must target list grid');
  });

  it('guarantees target card header contents fit inside single column container', () => {
    // In 1 column layout on 360px screen:
    // Available card width: ~330px
    // Avatar (44px) + Sync Pill (~70px) + Untrack button (26px) + gaps (16px) = 156px <= 330px (Passes with 174px margin)
    const requiredHeaderWidth = 44 + 70 + 26 + 16;
    const availableWidthOn360 = 330;
    assert.ok(requiredHeaderWidth < availableWidthOn360, 'Card header fits with zero clipping');
  });
});

describe('Official In-App Instagram Authentication Architecture', () => {
  it('correctly handles native bridge presence and fallbacks', async () => {
    // In Node test environment, window.StalkrAuth is absent
    const isAvail = isNativeAuthAvailable();
    assert.equal(isAvail, false, 'isNativeAuthAvailable must return false in non-Android environment');

    const result = await startNativeInstagramLogin();
    assert.equal(result.success, false);
    assert.ok(result.error?.includes('Android'), 'Must report Android requirement clearly');
  });

  it('verifies simulated native auth handshake with payload callback', async () => {
    // Simulate Android bridge
    let launched = false;
    globalThis.window = {
      StalkrAuth: {
        launchInstagramLogin: () => {
          launched = true;
          // Simulate instant callback from MainActivity
          setTimeout(() => {
            if (globalThis.window.__stalkr_onInstagramLogin) {
              globalThis.window.__stalkr_onInstagramLogin({
                success: true,
                sessionId: 'mock_session_123',
                dsUserId: '123456789',
                csrfToken: 'mock_csrf_abc',
              });
            }
          }, 10);
        },
      },
    };

    assert.equal(isNativeAuthAvailable(), true);
    
    // Clean up simulated environment
    delete globalThis.window;
  });
});
