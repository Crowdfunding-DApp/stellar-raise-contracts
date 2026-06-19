export interface CreateCampaignInput {
  creatorPublicKey: string;
  title: string;
  description: string;
  goalAmount: number;
  deadline: number;
  tokenAddress: string;
  minimumContribution: number;
  bonusGoal?: number;
}

export interface CreateCampaignResult {
  campaignAddress: string;
}

export type CreateCampaign = (
  input: CreateCampaignInput,
) => Promise<CreateCampaignResult>;

export const createCampaign: CreateCampaign = async () => {
  throw new Error(
    "Campaign factory integration is not configured. Wire this adapter to Issue #10's Soroban client.",
  );
};
