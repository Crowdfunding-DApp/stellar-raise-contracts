import {
  CampaignValidationValues,
  dateStringToDeadlineTimestamp,
  hasCampaignValidationErrors,
  parseAmount,
  validateCampaign,
  validateCampaignField,
} from "./campaignValidation";

const now = new Date("2026-06-18T12:00:00.000Z");

const validValues: CampaignValidationValues = {
  title: "Community Solar",
  description: "Funding a community solar installation.",
  goalAmount: "1000",
  deadline: "2026-06-19",
  tokenAddress: "CBIELUBUIXM6GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
  minimumContribution: "25",
  bonusGoal: "1500",
};

describe("campaign validation", () => {
  it("accepts a valid campaign", () => {
    const errors = validateCampaign(validValues, now);
    expect(hasCampaignValidationErrors(errors)).toBe(false);
  });

  it("rejects a zero goal before submission", () => {
    const error = validateCampaignField(
      "goalAmount",
      { ...validValues, goalAmount: "0" },
      now,
    );

    expect(error).toBe("Goal must be greater than 0.");
  });

  it("rejects a deadline in the past before submission", () => {
    const error = validateCampaignField(
      "deadline",
      { ...validValues, deadline: "2026-06-17" },
      now,
    );

    expect(error).toBe("Deadline must be in the future.");
  });

  it("rejects a minimum contribution above the goal", () => {
    const error = validateCampaignField(
      "minimumContribution",
      { ...validValues, minimumContribution: "1001" },
      now,
    );

    expect(error).toBe("Minimum contribution cannot exceed the goal.");
  });

  it("rejects a bonus goal that is not greater than the primary goal", () => {
    const error = validateCampaignField(
      "bonusGoal",
      { ...validValues, bonusGoal: "1000" },
      now,
    );

    expect(error).toBe("Bonus goal must be greater than the goal.");
  });

  it("allows the optional bonus goal to be empty", () => {
    const errors = validateCampaign({ ...validValues, bonusGoal: "" }, now);
    expect(errors.bonusGoal).toBeUndefined();
  });

  it("marks all required fields as invalid when empty", () => {
    const errors = validateCampaign(
      {
        title: "",
        description: "",
        goalAmount: "",
        deadline: "",
        tokenAddress: "",
        minimumContribution: "",
        bonusGoal: "",
      },
      now,
    );

    expect(errors.title).toBe("This field is required.");
    expect(errors.description).toBe("This field is required.");
    expect(errors.goalAmount).toBe("This field is required.");
    expect(errors.deadline).toBe("This field is required.");
    expect(errors.tokenAddress).toBe("This field is required.");
    expect(errors.minimumContribution).toBe("This field is required.");
  });

  it("parses decimal amounts for XLM or token unit input", () => {
    expect(parseAmount("12.5")).toBe(12.5);
    expect(parseAmount("abc")).toBeNull();
  });

  it("encodes a date field as an end-of-day unix timestamp", () => {
    expect(dateStringToDeadlineTimestamp("2026-06-19")).toBe(
      Math.floor(new Date(2026, 5, 19, 23, 59, 59, 999).getTime() / 1000),
    );
  });
});
