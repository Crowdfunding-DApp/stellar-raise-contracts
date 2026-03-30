/**
 * @title Campaign Milestone Celebration NFTs Tests
 * @notice Validates security sanitization, NFT plan generation, and panel rendering.
 * @dev Uses jsdom and Testing Library for UI checks.
 */

import React from 'react';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom';
import {
  MilestoneNftPanel,
  MilestoneNftSecurity,
  NFT_MAX_TEXT_LENGTH,
  NFT_MILESTONE_THRESHOLDS,
  computeCampaignMilestoneNftPlan,
  type CampaignMilestoneNftInput,
} from './milestone_nfts';

const baseInput = (): CampaignMilestoneNftInput => ({
  campaignId: 'stellar_campaign_1',
  campaignTitle: 'Solar Farm Buildout',
  creatorAddress: 'GCREATOR123',
  raisedAmount: 760,
  goalAmount: 1000,
  totalBackers: 18,
  imageBaseUri: 'https://cdn.example.com/images',
  metadataBaseUri: 'https://cdn.example.com/metadata',
});

describe('MilestoneNftSecurity', () => {
  it('sanitizes text and truncates to max length', () => {
    const dirty = ` x <script>alert(1)</script> ${'a'.repeat(NFT_MAX_TEXT_LENGTH + 20)} `;
    const value = MilestoneNftSecurity.sanitizeText(dirty);
    expect(value).not.toContain('<');
    expect(value).not.toContain('>');
    expect(value.length).toBe(NFT_MAX_TEXT_LENGTH);
  });

  it('sanitizes URI and rejects unsupported schemes', () => {
    expect(MilestoneNftSecurity.sanitizeHttpUri('https://a.example/x/')).toBe(
      'https://a.example/x'
    );
    expect(MilestoneNftSecurity.sanitizeHttpUri('http://a.example')).toBe('http://a.example');
    expect(MilestoneNftSecurity.sanitizeHttpUri('javascript:alert(1)')).toBe('');
    expect(MilestoneNftSecurity.sanitizeHttpUri('data:text/html;base64,abc')).toBe('');
  });

  it('validates campaign id strict format', () => {
    expect(MilestoneNftSecurity.isSafeCampaignId('abc_123-xyz')).toBe(true);
    expect(MilestoneNftSecurity.isSafeCampaignId('../bad')).toBe(false);
    expect(MilestoneNftSecurity.isSafeCampaignId('bad id')).toBe(false);
  });
});

describe('computeCampaignMilestoneNftPlan', () => {
  it('computes achieved and pending milestones by funding percent', () => {
    const plan = computeCampaignMilestoneNftPlan(baseInput());
    expect(plan.percentFunded).toBe(76);
    expect(plan.achievedMilestones).toEqual([25, 50, 75]);
    expect(plan.pendingMilestones).toEqual([100]);
  });

  it('creates deterministic metadata entries with expected token ids', () => {
    const plan = computeCampaignMilestoneNftPlan(baseInput());
    expect(plan.metadataToMint.map((m) => m.tokenId)).toEqual([
      'stellar_campaign_1-25',
      'stellar_campaign_1-50',
      'stellar_campaign_1-75',
    ]);
    expect(plan.metadataToMint[2].rarity).toBe('epic');
  });

  it('assigns legendary rarity at 100%', () => {
    const plan = computeCampaignMilestoneNftPlan({
      ...baseInput(),
      raisedAmount: 1400,
    });
    expect(plan.achievedMilestones).toEqual([...NFT_MILESTONE_THRESHOLDS]);
    const last = plan.metadataToMint[plan.metadataToMint.length - 1];
    expect(last.milestonePercent).toBe(100);
    expect(last.rarity).toBe('legendary');
  });

  it('returns security checks and no metadata for invalid URI inputs', () => {
    const plan = computeCampaignMilestoneNftPlan({
      ...baseInput(),
      imageBaseUri: 'javascript:alert(1)',
      metadataBaseUri: 'ftp://example.com/meta',
    });
    expect(plan.securityChecks.length).toBeGreaterThan(0);
    expect(plan.metadataToMint).toEqual([]);
  });

  it('returns security checks for invalid campaign id and zero goal', () => {
    const plan = computeCampaignMilestoneNftPlan({
      ...baseInput(),
      campaignId: '../../oops',
      goalAmount: 0,
    });
    expect(plan.securityChecks).toEqual(
      expect.arrayContaining([
        'Invalid campaign id format',
        'Goal must be greater than zero for milestone NFTs',
      ])
    );
    expect(plan.metadataToMint).toEqual([]);
  });

  it('clamps negative and non-finite values to safe defaults', () => {
    const plan = computeCampaignMilestoneNftPlan({
      ...baseInput(),
      raisedAmount: Number.POSITIVE_INFINITY,
      goalAmount: -100,
      totalBackers: -3,
    });
    expect(plan.percentFunded).toBe(0);
    expect(plan.achievedMilestones).toEqual([]);
    expect(plan.metadataToMint).toEqual([]);
  });
});

describe('MilestoneNftPanel', () => {
  it('renders unlocked NFT list when security checks pass', () => {
    render(<MilestoneNftPanel input={baseInput()} />);
    expect(screen.getByTestId('milestone-nft-percent')).toHaveTextContent('76.0%');
    expect(screen.getByTestId('milestone-nft-achieved')).toHaveTextContent('25, 50, 75');
    expect(screen.getByTestId('milestone-nft-list')).toBeInTheDocument();
    expect(screen.getByTestId('milestone-nft-stellar_campaign_1-50')).toHaveTextContent(
      '50% Milestone NFT'
    );
  });

  it('renders security alert for invalid configuration', () => {
    render(
      <MilestoneNftPanel
        input={{
          ...baseInput(),
          metadataBaseUri: 'javascript:bad',
        }}
      />
    );
    expect(screen.getByTestId('milestone-nft-security-alert')).toBeInTheDocument();
    expect(screen.queryByTestId('milestone-nft-list')).not.toBeInTheDocument();
  });

  it('supports custom test id and className', () => {
    const { container } = render(
      <MilestoneNftPanel input={baseInput()} testId="custom-nft-panel" className="panel-wrap" />
    );
    expect(screen.getByTestId('custom-nft-panel')).toBeInTheDocument();
    expect(container.firstChild).toHaveClass('milestone-nft-panel', 'panel-wrap');
  });
});
