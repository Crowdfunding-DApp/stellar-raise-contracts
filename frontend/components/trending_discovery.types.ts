/**
 * Types for Trending Discovery component.
 * Secure: Fully typed to prevent runtime errors.
 * @file trending_discovery.types.ts
 */

export interface TrendingCampaign {
  id: string;
  name: string;
  funded: number;
  goal: number;
  backers: number;
  trendScore: number; // 0-100
  hot?: boolean;
}
