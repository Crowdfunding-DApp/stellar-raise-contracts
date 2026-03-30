/**
 * @title Campaign Milestone Celebration NFTs
 * @notice Secure NFT metadata preparation and frontend-ready mint plan generation
 * @dev Pure functions plus a lightweight React component for previewing milestone collectible drops.
 * @author Stellar Raise Team
 * @version 1.0.0
 */

import React, { useMemo } from 'react';

/** @notice Maximum length for user-facing text used in metadata */
export const NFT_MAX_TEXT_LENGTH = 120;

/** @notice Ordered campaign milestone thresholds used to unlock celebration NFTs */
export const NFT_MILESTONE_THRESHOLDS = [25, 50, 75, 100] as const;

/** @notice Supported rarity levels for campaign celebration NFTs */
export type NftRarity = 'common' | 'rare' | 'epic' | 'legendary';

/**
 * @notice Raw campaign progress payload used to derive NFT milestones
 */
export interface CampaignMilestoneNftInput {
  campaignId: string;
  campaignTitle: string;
  creatorAddress: string;
  raisedAmount: number;
  goalAmount: number;
  totalBackers: number;
  imageBaseUri: string;
  metadataBaseUri: string;
}

/**
 * @notice One celebration NFT metadata object for pinning or contract mint payloads
 * @dev Kept deterministic so UI previews match minted content.
 */
export interface CelebrationNftMetadata {
  tokenId: string;
  campaignId: string;
  milestonePercent: number;
  name: string;
  description: string;
  image: string;
  externalUrl: string;
  attributes: Array<{ trait_type: string; value: string | number }>;
  rarity: NftRarity;
}

/**
 * @notice Frontend action plan for NFTs available to mint for achieved milestones
 */
export interface MilestoneNftMintPlan {
  campaignId: string;
  campaignTitle: string;
  percentFunded: number;
  achievedMilestones: number[];
  pendingMilestones: number[];
  metadataToMint: CelebrationNftMetadata[];
  securityChecks: string[];
}

/**
 * @title MilestoneNftSecurity
 * @notice Security-oriented validation and sanitization helpers
 */
export class MilestoneNftSecurity {
  /**
   * @notice Sanitizes display text for metadata fields and frontend rendering
   * @dev Removes control characters and angle-bracket fragments.
   */
  static sanitizeText(input: string): string {
    if (typeof input !== 'string') {
      return '';
    }
    const withoutControls = input.replace(/[\u0000-\u001F\u007F]/g, '');
    const withoutTags = withoutControls.replace(/<[^>]*>/g, '');
    return withoutTags.replace(/\s+/g, ' ').trim().slice(0, NFT_MAX_TEXT_LENGTH);
  }

  /**
   * @notice Restricts URI values to http(s) scheme and strips trailing slashes
   * @dev Prevents javascript/data URI injection in metadata links.
   */
  static sanitizeHttpUri(uri: string): string {
    if (typeof uri !== 'string') {
      return '';
    }
    const trimmed = uri.trim().replace(/\/+$/, '');
    if (!/^https?:\/\/[a-zA-Z0-9.-]+(?::\d+)?(\/.*)?$/.test(trimmed)) {
      return '';
    }
    return trimmed;
  }

  /**
   * @notice Validates campaign id for deterministic token id creation
   */
  static isSafeCampaignId(campaignId: string): boolean {
    return /^[a-zA-Z0-9_-]{1,64}$/.test(campaignId);
  }

  /**
   * @notice Coerces numeric values to finite non-negative values
   */
  static clampNonNegative(value: number): number {
    if (!Number.isFinite(value) || value < 0) {
      return 0;
    }
    return value;
  }
}

function clampPercent(percent: number): number {
  return Math.max(0, Math.min(100, percent));
}

function rarityForMilestone(milestonePercent: number): NftRarity {
  if (milestonePercent >= 100) {
    return 'legendary';
  }
  if (milestonePercent >= 75) {
    return 'epic';
  }
  if (milestonePercent >= 50) {
    return 'rare';
  }
  return 'common';
}

function buildMetadataEntry(
  safeCampaignId: string,
  safeTitle: string,
  milestonePercent: number,
  totalBackers: number,
  imageBaseUri: string,
  metadataBaseUri: string
): CelebrationNftMetadata {
  const rarity = rarityForMilestone(milestonePercent);
  const tokenId = `${safeCampaignId}-${milestonePercent}`;
  return {
    tokenId,
    campaignId: safeCampaignId,
    milestonePercent,
    name: `${safeTitle} - ${milestonePercent}% Milestone NFT`,
    description: `Celebration collectible for reaching ${milestonePercent}% funding progress.`,
    image: `${imageBaseUri}/${safeCampaignId}/${milestonePercent}.png`,
    externalUrl: `${metadataBaseUri}/${safeCampaignId}/${milestonePercent}.json`,
    attributes: [
      { trait_type: 'Milestone', value: `${milestonePercent}%` },
      { trait_type: 'Rarity', value: rarity },
      { trait_type: 'Backers', value: totalBackers },
    ],
    rarity,
  };
}

/**
 * @notice Generates secure, deterministic NFT mint plan for campaign milestones
 * @dev Returns empty mint list on invalid ids or URI configuration.
 */
export function computeCampaignMilestoneNftPlan(
  input: CampaignMilestoneNftInput
): MilestoneNftMintPlan {
  const safeCampaignId = MilestoneNftSecurity.sanitizeText(input.campaignId);
  const safeTitle = MilestoneNftSecurity.sanitizeText(input.campaignTitle) || 'Campaign';
  const goal = MilestoneNftSecurity.clampNonNegative(input.goalAmount);
  const raised = MilestoneNftSecurity.clampNonNegative(input.raisedAmount);
  const backers = Math.floor(MilestoneNftSecurity.clampNonNegative(input.totalBackers));
  const imageBaseUri = MilestoneNftSecurity.sanitizeHttpUri(input.imageBaseUri);
  const metadataBaseUri = MilestoneNftSecurity.sanitizeHttpUri(input.metadataBaseUri);

  const percentFunded = goal > 0 ? clampPercent((raised / goal) * 100) : 0;
  const achievedMilestones = NFT_MILESTONE_THRESHOLDS.filter(
    (threshold) => percentFunded >= threshold
  );
  const pendingMilestones = NFT_MILESTONE_THRESHOLDS.filter(
    (threshold) => percentFunded < threshold
  );

  const securityChecks: string[] = [];
  if (!MilestoneNftSecurity.isSafeCampaignId(safeCampaignId)) {
    securityChecks.push('Invalid campaign id format');
  }
  if (!imageBaseUri || !metadataBaseUri) {
    securityChecks.push('Invalid base URI configuration (http/https required)');
  }
  if (goal <= 0) {
    securityChecks.push('Goal must be greater than zero for milestone NFTs');
  }

  const metadataToMint =
    securityChecks.length > 0
      ? []
      : achievedMilestones.map((milestone) =>
          buildMetadataEntry(
            safeCampaignId,
            safeTitle,
            milestone,
            backers,
            imageBaseUri,
            metadataBaseUri
          )
        );

  return {
    campaignId: safeCampaignId,
    campaignTitle: safeTitle,
    percentFunded,
    achievedMilestones,
    pendingMilestones,
    metadataToMint,
    securityChecks,
  };
}

export interface MilestoneNftPanelProps {
  input: CampaignMilestoneNftInput;
  className?: string;
  testId?: string;
}

/**
 * @title MilestoneNftPanel
 * @notice Frontend component to preview celebration NFT unlock state and mint payload
 */
export const MilestoneNftPanel: React.FC<MilestoneNftPanelProps> = ({
  input,
  className,
  testId = 'milestone-nft-panel',
}) => {
  const plan = useMemo(() => computeCampaignMilestoneNftPlan(input), [input]);
  const rootClass = ['milestone-nft-panel', className].filter(Boolean).join(' ');

  return (
    <section className={rootClass} data-testid={testId} aria-label="Milestone celebration NFTs">
      <h2>Milestone celebration NFTs</h2>
      <p data-testid="milestone-nft-percent">
        Campaign funded: {plan.percentFunded.toFixed(1)}%
      </p>

      {plan.securityChecks.length > 0 ? (
        <div role="alert" data-testid="milestone-nft-security-alert">
          <p>Security configuration issues detected:</p>
          <ul>
            {plan.securityChecks.map((check) => (
              <li key={check}>{check}</li>
            ))}
          </ul>
        </div>
      ) : (
        <>
          <p data-testid="milestone-nft-achieved">
            Achieved milestones: {plan.achievedMilestones.join(', ') || 'None'}
          </p>
          <p data-testid="milestone-nft-pending">
            Pending milestones: {plan.pendingMilestones.join(', ') || 'None'}
          </p>
          <ul data-testid="milestone-nft-list">
            {plan.metadataToMint.map((nft) => (
              <li key={nft.tokenId} data-testid={`milestone-nft-${nft.tokenId}`}>
                <strong>{nft.name}</strong> ({nft.rarity}) - {nft.externalUrl}
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  );
};

export default MilestoneNftPanel;
