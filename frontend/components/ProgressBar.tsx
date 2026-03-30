import React from 'react';

/**
 * Reusable ProgressBar component.
 * @param progress - 0-100 percentage.
 */
interface ProgressBarProps {
  progress: number;
}

const ProgressBar: React.FC<ProgressBarProps> = ({ progress }) => (
  <div className="progress-bar">
    <div 
      className="progress-fill" 
      style={{ width: `${Math.min(Math.max(progress, 0), 100)}%` }}
      role="progressbar"
      aria-valuenow={progress}
      aria-valuemin={0}
      aria-valuemax={100}
    />
  </div>
);

export default ProgressBar;
