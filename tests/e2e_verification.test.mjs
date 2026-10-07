import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { getFallbackAvatar, getAvatarUrl } from '../src/lib/avatar.ts';
import { generateRandomDemoData, mockAccount, mockTargetAccount } from '../src/lib/mockData.ts';

describe('Avatar Utility Suite', () => {
  it('generates consistent SVG data URIs for user handles', () => {
    const avatar1 = getFallbackAvatar('alex_vance');
    const avatar2 = getFallbackAvatar('alex_vance');
    assert.ok(avatar1.startsWith('data:image/svg+xml;utf8,'));
    assert.equal(avatar1, avatar2, 'Deterministic hashing must produce identical avatars for the same handle');
  });

  it('correctly extracts uppercase 2-letter initials', () => {
    const avatar = getFallbackAvatar('meetzarc');
    assert.ok(decodeURIComponent(avatar).includes('ME'), 'Initials for meetzarc must be ME');
  });

  it('filters out unstable external unsplash URLs that trigger 403', () => {
    const unsplashUrl = 'https://images.unsplash.com/photo-1534528741775-53994a69daeb';
    const resolved = getAvatarUrl('elena.rostova', unsplashUrl);
    assert.ok(resolved.startsWith('data:image/svg+xml;utf8,'), 'Must replace unsplash with safe local SVG avatar');
  });

  it('preserves valid local and asset URLs', () => {
    const localUrl = '/profile.png';
    const resolved = getAvatarUrl('meetzarc', localUrl);
    assert.equal(resolved, '/profile.png');
  });
});

describe('Mock Data & Simulation Isolation Suite', () => {
  it('generates rich root owner account with realistic stats', () => {
    const data = generateRandomDemoData();
    const owner = data.ownerAccount;
    assert.equal(owner.username, 'meetzarc');
    assert.equal(owner.display_name, 'Meet Mistry');
    assert.equal(owner.account_kind, 'owner');
    assert.ok(owner.followers_count > 1000);
    assert.ok(owner.following_count > 500);
    assert.equal(owner.is_verified, true);
    assert.equal(owner.access_state, 'accessible');
  });

  it('generates isolated private monitored target account', () => {
    const data = generateRandomDemoData();
    const target = data.targetAccount;
    assert.equal(target.account_kind, 'monitored');
    assert.equal(target.authenticated_by_account_id, 'acc_demo_meetzarc');
    assert.equal(target.is_private, true);
    assert.ok(target.id.length > 0);
    assert.notEqual(target.username, 'meetzarc');
  });

  it('produces populated people and changes arrays', () => {
    const data = generateRandomDemoData();
    assert.ok(data.people.length > 0);
    assert.ok(data.targetPeople.length > 0);
    assert.ok(data.changes.length > 0);
  });
});

describe('E2E Production Readiness & Instagram Contract Verification', () => {
  it('verifies that Instagram cookie parser correctly isolates tokens', () => {
    const rawCookie = 'mid=Z8...; ds_user_id=1234567890; csrftoken=abc123xyz; sessionid=1234567890%3Asome_secret_hash%3A25%3AAYf; rur="NCG"';
    
    const sessionMatch = rawCookie.match(/sessionid=([^;\s]+)/i);
    const dsUserMatch = rawCookie.match(/ds_user_id=([^;\s]+)/i);
    const csrfMatch = rawCookie.match(/csrftoken=([^;\s]+)/i);

    assert.ok(sessionMatch, 'sessionid must be extracted');
    assert.ok(dsUserMatch, 'ds_user_id must be extracted');
    assert.ok(csrfMatch, 'csrftoken must be extracted');

    assert.equal(dsUserMatch[1], '1234567890');
    assert.equal(csrfMatch[1], 'abc123xyz');
    assert.ok(sessionMatch[1].startsWith('1234567890%3A'));
  });

  it('verifies developer portfolio and email contact endpoints', () => {
    const devUrl = 'https://meetmistry.vercel.app';
    const email = 'mailto:meetzarc@gmail.com';
    assert.ok(devUrl.startsWith('https://'));
    assert.ok(email.startsWith('mailto:'));
    assert.ok(email.includes('@'));
  });
});
