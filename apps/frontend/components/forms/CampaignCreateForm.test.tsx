import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import CampaignCreateForm from "./CampaignCreateForm";
import { CreateCampaign } from "../../utils/sorobanCampaignFactory";

function fillValidForm() {
  fireEvent.change(screen.getByLabelText("Campaign title"), {
    target: { value: "Community Solar" },
  });
  fireEvent.change(screen.getByLabelText("Campaign description"), {
    target: { value: "Funding a community solar installation." },
  });
  fireEvent.change(screen.getByLabelText("Goal amount"), {
    target: { value: "1000" },
  });
  fireEvent.change(screen.getByLabelText("Deadline"), {
    target: { value: "2099-06-19" },
  });
  fireEvent.change(screen.getByLabelText("Token contract address"), {
    target: {
      value: "CBIELUBUIXM6GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    },
  });
  fireEvent.change(screen.getByLabelText("Minimum contribution"), {
    target: { value: "25" },
  });
  fireEvent.change(screen.getByLabelText("Bonus goal"), {
    target: { value: "1500" },
  });
}

describe("CampaignCreateForm", () => {
  it("renders all required campaign fields with accessible labels", () => {
    render(<CampaignCreateForm creatorPublicKey="GCREATOR" />);

    expect(screen.getByLabelText("Campaign title")).toBeInTheDocument();
    expect(screen.getByLabelText("Campaign description")).toBeInTheDocument();
    expect(screen.getByLabelText("Goal amount")).toBeInTheDocument();
    expect(screen.getByLabelText("Deadline")).toBeInTheDocument();
    expect(screen.getByLabelText("Token contract address")).toBeInTheDocument();
    expect(screen.getByLabelText("Minimum contribution")).toBeInTheDocument();
    expect(screen.getByLabelText("Bonus goal")).toBeInTheDocument();
  });

  it("shows validation errors before submission when fields are blurred", () => {
    render(<CampaignCreateForm creatorPublicKey="GCREATOR" />);

    fireEvent.change(screen.getByLabelText("Goal amount"), {
      target: { value: "0" },
    });
    fireEvent.blur(screen.getByLabelText("Goal amount"));

    fireEvent.change(screen.getByLabelText("Deadline"), {
      target: { value: "2020-01-01" },
    });
    fireEvent.blur(screen.getByLabelText("Deadline"));

    expect(
      screen.getByText("Goal must be greater than 0."),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Deadline must be in the future."),
    ).toBeInTheDocument();
  });

  it("does not submit when validation errors are present", async () => {
    const createCampaign = vi.fn<CreateCampaign>();

    render(
      <CampaignCreateForm
        creatorPublicKey="GCREATOR"
        createCampaign={createCampaign}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Create campaign" }));

    expect(await screen.findAllByText("This field is required.")).toHaveLength(
      6,
    );
    expect(createCampaign).not.toHaveBeenCalled();
  });

  it("submits a valid form through the factory call and shows success", async () => {
    const createCampaign = vi
      .fn<CreateCampaign>()
      .mockResolvedValue({ campaignAddress: "CCAMPAIGN" });

    render(
      <CampaignCreateForm
        creatorPublicKey="GCREATOR"
        createCampaign={createCampaign}
      />,
    );

    fillValidForm();
    fireEvent.click(screen.getByRole("button", { name: "Create campaign" }));

    await waitFor(() => {
      expect(createCampaign).toHaveBeenCalledTimes(1);
    });

    expect(createCampaign).toHaveBeenCalledWith(
      expect.objectContaining({
        creatorPublicKey: "GCREATOR",
        title: "Community Solar",
        goalAmount: 1000,
        tokenAddress:
          "CBIELUBUIXM6GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        minimumContribution: 25,
        bonusGoal: 1500,
      }),
    );
    expect(
      await screen.findByText("Campaign created at CCAMPAIGN."),
    ).toBeTruthy();
  });

  it("shows an error when the factory call fails", async () => {
    const createCampaign = vi
      .fn<CreateCampaign>()
      .mockRejectedValue(new Error("Wallet rejected the transaction."));

    render(
      <CampaignCreateForm
        creatorPublicKey="GCREATOR"
        createCampaign={createCampaign}
      />,
    );

    fillValidForm();
    fireEvent.click(screen.getByRole("button", { name: "Create campaign" }));

    expect(
      await screen.findByText("Wallet rejected the transaction."),
    ).toBeInTheDocument();
  });
});
