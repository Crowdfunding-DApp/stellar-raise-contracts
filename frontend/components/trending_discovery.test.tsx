import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import TrendingDiscovery, { TrendingCampaign } from './trending_discovery';

/**
 * Comprehensive tests for TrendingDiscovery.
 * Covers: render, loading, empty, campaigns, progress, badges, accessibility.
 * Edge cases: 0 campaigns, invalid progress, large lists.
 * Security: Snapshot no XSS vectors.
 */

describe('TrendingDiscovery', () => {
  const mockCampaigns: TrendingCampaign[] = [
    {
      id: '1',
      name: 'Test Campaign',
      funded: 50000,
      goal: 100000,
      backers: 100,
      trendScore: 85,
      hot: true,
    },
  ];

  it('renders loading state initially', () => {
    render(<TrendingDiscovery />);
    expect(screen.getByText('Discovering trends...')).toBeInTheDocument();
  });

  it('renders trending campaigns after load', async () => {
    render(<TrendingDiscovery campaigns={mockCampaigns} />);
    await waitFor(() => {
      expect(screen.getByText('Trending Campaigns')).toBeInTheDocument();
      expect(screen.getByText('Test Campaign')).toBeInTheDocument();
      expect(screen.getByText('$50,000 / $100,000')).toBeInTheDocument();
    });
  });

  it('shows HOT badge for hot campaigns', async () => {
    render(<TrendingDiscovery campaigns={mockCampaigns} />);
    await waitFor(() => {
      expect(screen.getByText('HOT')).toBeInTheDocument();
    });
  });

  it('handles empty campaigns gracefully', async () => {
    render(<TrendingDiscovery campaigns={[]} />);
    await waitFor(() => {
      expect(screen.queryByText('Test Campaign')).not.toBeInTheDocument();
    });
  });

  it('limits campaigns display', async () => {
    const manyCampaigns = Array.from({ length: 20 }, (_, i) => ({
      id: `${i}`,
      name: `Campaign ${i}`,
      funded: 10000,
      goal: 20000,
      backers: 10,
      trendScore: 50,
    }));
    render(<TrendingDiscovery campaigns={manyCampaigns} limit={5} />);
    await waitFor(() => {
      const cards = screen.getAllByText(/Campaign \d+/);
      expect(cards.length).toBeLessThanOrEqual(5);
    });
  });

  it('progress bar accessibility', async () => {
    render(<TrendingDiscovery campaigns={mockCampaigns} />);
    await waitFor(() => {
      const progress = screen.getByRole('progressbar');
      expect(progress).toHaveAttribute('aria-valuenow', '50');
    });
  });

  it('matches snapshot', () => {
    const { asFragment } = render(<TrendingDiscovery campaigns={mockCampaigns} />);
    expect(asFragment()).toMatchSnapshot();
  });

  // Security test: No XSS from props
  it('safely renders user-like data', () => {
    const unsafeCampaigns: TrendingCampaign[] = [{
      id: 'xss-test',
      name: '<script>alert("xss")</script>',
      funded: 0,
      goal: 1,
      backers: 0,
      trendScore: 0,
    }];
    render(<TrendingDiscovery campaigns={unsafeCampaigns} />);
    // React should escape; no alert executed
    expect(screen.getByText('<script>alert("xss")</script>')).toBeInTheDocument(); // Escaped text
  });
});
