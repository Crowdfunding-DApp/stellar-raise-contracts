export interface CampaignValidationValues {
  title: string;
  description: string;
  goalAmount: string;
  deadline: string;
  tokenAddress: string;
  minimumContribution: string;
  bonusGoal: string;
}

export type CampaignValidationField = keyof CampaignValidationValues;

export type CampaignValidationErrors = Partial<
  Record<CampaignValidationField, string>
>;

const REQUIRED_MESSAGE = "This field is required.";
const CONTRACT_ADDRESS_PATTERN = /^C[A-Z2-7]{10,}$/;

export function parseAmount(value: string): number | null {
  const trimmedValue = value.trim();

  if (!trimmedValue) {
    return null;
  }

  const amount = Number(trimmedValue);

  if (!Number.isFinite(amount)) {
    return null;
  }

  return amount;
}

export function dateStringToDeadlineTimestamp(value: string): number | null {
  if (!value) {
    return null;
  }

  const [year, month, day] = value.split("-").map(Number);

  if (!year || !month || !day) {
    return null;
  }

  return Math.floor(
    new Date(year, month - 1, day, 23, 59, 59, 999).getTime() / 1000,
  );
}

export function validateCampaignField(
  field: CampaignValidationField,
  values: CampaignValidationValues,
  now: Date = new Date(),
): string | undefined {
  const value = values[field].trim();

  if (field !== "bonusGoal" && !value) {
    return REQUIRED_MESSAGE;
  }

  if (field === "bonusGoal" && !value) {
    return undefined;
  }

  if (field === "goalAmount") {
    const goalAmount = parseAmount(value);

    if (goalAmount === null) {
      return "Goal must be a valid number.";
    }

    if (goalAmount <= 0) {
      return "Goal must be greater than 0.";
    }
  }

  if (field === "deadline") {
    const deadlineTimestamp = dateStringToDeadlineTimestamp(value);

    if (deadlineTimestamp === null) {
      return "Deadline must be a valid date.";
    }

    if (deadlineTimestamp <= Math.floor(now.getTime() / 1000)) {
      return "Deadline must be in the future.";
    }
  }

  if (field === "tokenAddress" && !CONTRACT_ADDRESS_PATTERN.test(value)) {
    return "Token contract address must be a valid Stellar contract address.";
  }

  if (field === "minimumContribution") {
    const minimumContribution = parseAmount(value);
    const goalAmount = parseAmount(values.goalAmount);

    if (minimumContribution === null) {
      return "Minimum contribution must be a valid number.";
    }

    if (minimumContribution <= 0) {
      return "Minimum contribution must be greater than 0.";
    }

    if (goalAmount !== null && minimumContribution > goalAmount) {
      return "Minimum contribution cannot exceed the goal.";
    }
  }

  if (field === "bonusGoal") {
    const bonusGoal = parseAmount(value);
    const goalAmount = parseAmount(values.goalAmount);

    if (bonusGoal === null) {
      return "Bonus goal must be a valid number.";
    }

    if (goalAmount !== null && bonusGoal <= goalAmount) {
      return "Bonus goal must be greater than the goal.";
    }
  }

  return undefined;
}

export function validateCampaign(
  values: CampaignValidationValues,
  now: Date = new Date(),
): CampaignValidationErrors {
  return (Object.keys(values) as CampaignValidationField[]).reduce(
    (errors, field) => {
      const error = validateCampaignField(field, values, now);

      if (error) {
        errors[field] = error;
      }

      return errors;
    },
    {} as CampaignValidationErrors,
  );
}

export function hasCampaignValidationErrors(
  errors: CampaignValidationErrors,
): boolean {
  return Object.values(errors).some(Boolean);
}
