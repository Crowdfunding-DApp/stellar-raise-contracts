import { ChangeEvent, FormEvent, useMemo, useState } from "react";
import {
  CampaignValidationErrors,
  CampaignValidationField,
  CampaignValidationValues,
  dateStringToDeadlineTimestamp,
  hasCampaignValidationErrors,
  parseAmount,
  validateCampaign,
  validateCampaignField,
} from "../../utils/campaignValidation";
import {
  createCampaign as defaultCreateCampaign,
  CreateCampaign,
} from "../../utils/sorobanCampaignFactory";
import "./Forms.css";

interface CampaignCreateFormProps {
  creatorPublicKey: string;
  createCampaign?: CreateCampaign;
}

type SubmissionState = "idle" | "loading" | "success" | "error";

const initialValues: CampaignValidationValues = {
  title: "",
  description: "",
  goalAmount: "",
  deadline: "",
  tokenAddress: "",
  minimumContribution: "",
  bonusGoal: "",
};

const fieldOrder: CampaignValidationField[] = [
  "title",
  "description",
  "goalAmount",
  "deadline",
  "tokenAddress",
  "minimumContribution",
  "bonusGoal",
];

function buildDescribedBy(
  field: CampaignValidationField,
  error?: string,
  hintId?: string,
) {
  const ids = [hintId, error ? `${field}-error` : undefined].filter(Boolean);
  return ids.length > 0 ? ids.join(" ") : undefined;
}

export default function CampaignCreateForm({
  creatorPublicKey,
  createCampaign = defaultCreateCampaign,
}: CampaignCreateFormProps) {
  const [values, setValues] = useState<CampaignValidationValues>(initialValues);
  const [touched, setTouched] = useState<
    Partial<Record<CampaignValidationField, boolean>>
  >({});
  const [errors, setErrors] = useState<CampaignValidationErrors>({});
  const [submissionState, setSubmissionState] =
    useState<SubmissionState>("idle");
  const [statusMessage, setStatusMessage] = useState("");

  const isSubmitting = submissionState === "loading";

  const statusClassName = useMemo(() => {
    if (submissionState === "idle") {
      return "";
    }

    return `form__status form__status--${submissionState === "loading" ? "loading" : submissionState}`;
  }, [submissionState]);

  const updateField =
    (field: CampaignValidationField) =>
    (
      event: ChangeEvent<HTMLInputElement> | ChangeEvent<HTMLTextAreaElement>,
    ) => {
      const nextValues = { ...values, [field]: event.target.value };
      setValues(nextValues);

      if (touched[field]) {
        setErrors((currentErrors) => ({
          ...currentErrors,
          [field]: validateCampaignField(field, nextValues),
        }));
      }

      if (submissionState !== "idle") {
        setSubmissionState("idle");
        setStatusMessage("");
      }
    };

  const markTouched = (field: CampaignValidationField) => () => {
    setTouched((currentTouched) => ({ ...currentTouched, [field]: true }));
    setErrors((currentErrors) => ({
      ...currentErrors,
      [field]: validateCampaignField(field, values),
    }));
  };

  const renderError = (field: CampaignValidationField) =>
    errors[field] ? (
      <p className="form__error" id={`${field}-error`} role="alert">
        {errors[field]}
      </p>
    ) : null;

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const nextErrors = validateCampaign(values);
    setTouched(
      fieldOrder.reduce(
        (nextTouched, field) => ({ ...nextTouched, [field]: true }),
        {} as Record<CampaignValidationField, boolean>,
      ),
    );
    setErrors(nextErrors);

    if (hasCampaignValidationErrors(nextErrors)) {
      setSubmissionState("error");
      setStatusMessage(
        "Fix the highlighted fields before creating a campaign.",
      );
      return;
    }

    const goalAmount = parseAmount(values.goalAmount);
    const deadline = dateStringToDeadlineTimestamp(values.deadline);
    const minimumContribution = parseAmount(values.minimumContribution);
    const bonusGoal = parseAmount(values.bonusGoal);

    if (
      goalAmount === null ||
      deadline === null ||
      minimumContribution === null
    ) {
      return;
    }

    setSubmissionState("loading");
    setStatusMessage("Submitting campaign transaction...");

    try {
      const result = await createCampaign({
        creatorPublicKey,
        title: values.title.trim(),
        description: values.description.trim(),
        goalAmount,
        deadline,
        tokenAddress: values.tokenAddress.trim(),
        minimumContribution,
        ...(bonusGoal === null ? {} : { bonusGoal }),
      });

      setSubmissionState("success");
      setStatusMessage(`Campaign created at ${result.campaignAddress}.`);
    } catch (error) {
      setSubmissionState("error");
      setStatusMessage(
        error instanceof Error
          ? error.message
          : "Campaign creation failed. Try again.",
      );
    }
  };

  return (
    <form className="form" noValidate onSubmit={handleSubmit}>
      <div className="form__group">
        <label className="form__label form__label--required" htmlFor="title">
          Campaign title
        </label>
        <input
          aria-describedby={buildDescribedBy("title", errors.title)}
          aria-invalid={Boolean(errors.title)}
          className={`form__input${errors.title ? " form__input--error" : ""}`}
          disabled={isSubmitting}
          id="title"
          name="title"
          onBlur={markTouched("title")}
          onChange={updateField("title")}
          type="text"
          value={values.title}
        />
        {renderError("title")}
      </div>

      <div className="form__group">
        <label
          className="form__label form__label--required"
          htmlFor="description"
        >
          Campaign description
        </label>
        <textarea
          aria-describedby={buildDescribedBy("description", errors.description)}
          aria-invalid={Boolean(errors.description)}
          className={`form__textarea${errors.description ? " form__textarea--error" : ""}`}
          disabled={isSubmitting}
          id="description"
          name="description"
          onBlur={markTouched("description")}
          onChange={updateField("description")}
          value={values.description}
        />
        {renderError("description")}
      </div>

      <div className="form__group">
        <label
          className="form__label form__label--required"
          htmlFor="goalAmount"
        >
          Goal amount
        </label>
        <input
          aria-describedby={buildDescribedBy("goalAmount", errors.goalAmount)}
          aria-invalid={Boolean(errors.goalAmount)}
          className={`form__input${errors.goalAmount ? " form__input--error" : ""}`}
          disabled={isSubmitting}
          id="goalAmount"
          inputMode="decimal"
          min="0"
          name="goalAmount"
          onBlur={markTouched("goalAmount")}
          onChange={updateField("goalAmount")}
          step="any"
          type="number"
          value={values.goalAmount}
        />
        {renderError("goalAmount")}
      </div>

      <div className="form__group">
        <label className="form__label form__label--required" htmlFor="deadline">
          Deadline
        </label>
        <input
          aria-describedby={buildDescribedBy("deadline", errors.deadline)}
          aria-invalid={Boolean(errors.deadline)}
          className={`form__input${errors.deadline ? " form__input--error" : ""}`}
          disabled={isSubmitting}
          id="deadline"
          name="deadline"
          onBlur={markTouched("deadline")}
          onChange={updateField("deadline")}
          type="date"
          value={values.deadline}
        />
        {renderError("deadline")}
      </div>

      <div className="form__group">
        <label
          className="form__label form__label--required"
          htmlFor="tokenAddress"
        >
          Token contract address
        </label>
        <input
          aria-describedby={buildDescribedBy(
            "tokenAddress",
            errors.tokenAddress,
            "tokenAddress-hint",
          )}
          aria-invalid={Boolean(errors.tokenAddress)}
          className={`form__input${errors.tokenAddress ? " form__input--error" : ""}`}
          disabled={isSubmitting}
          id="tokenAddress"
          name="tokenAddress"
          onBlur={markTouched("tokenAddress")}
          onChange={updateField("tokenAddress")}
          type="text"
          value={values.tokenAddress}
        />
        <p className="form__hint" id="tokenAddress-hint">
          Stellar contract address beginning with C.
        </p>
        {renderError("tokenAddress")}
      </div>

      <div className="form__group">
        <label
          className="form__label form__label--required"
          htmlFor="minimumContribution"
        >
          Minimum contribution
        </label>
        <input
          aria-describedby={buildDescribedBy(
            "minimumContribution",
            errors.minimumContribution,
          )}
          aria-invalid={Boolean(errors.minimumContribution)}
          className={`form__input${errors.minimumContribution ? " form__input--error" : ""}`}
          disabled={isSubmitting}
          id="minimumContribution"
          inputMode="decimal"
          min="0"
          name="minimumContribution"
          onBlur={markTouched("minimumContribution")}
          onChange={updateField("minimumContribution")}
          step="any"
          type="number"
          value={values.minimumContribution}
        />
        {renderError("minimumContribution")}
      </div>

      <div className="form__group">
        <label className="form__label" htmlFor="bonusGoal">
          Bonus goal
        </label>
        <input
          aria-describedby={buildDescribedBy("bonusGoal", errors.bonusGoal)}
          aria-invalid={Boolean(errors.bonusGoal)}
          className={`form__input${errors.bonusGoal ? " form__input--error" : ""}`}
          disabled={isSubmitting}
          id="bonusGoal"
          inputMode="decimal"
          min="0"
          name="bonusGoal"
          onBlur={markTouched("bonusGoal")}
          onChange={updateField("bonusGoal")}
          step="any"
          type="number"
          value={values.bonusGoal}
        />
        {renderError("bonusGoal")}
      </div>

      {statusMessage ? (
        <div className={statusClassName} role="status">
          {statusMessage}
        </div>
      ) : null}

      <button
        aria-busy={isSubmitting}
        className="btn btn--primary btn--full"
        disabled={isSubmitting}
        type="submit"
      >
        {isSubmitting ? "Creating campaign..." : "Create campaign"}
      </button>
    </form>
  );
}
