import React from 'react';

/**
 * Reusable Badge component.
 * Secure: Sanitized text prop.
 */
interface BadgeProps {
  text: string;
  variant?: 'fire' | 'new' | 'hot';
}

const Badge: React.FC<BadgeProps> = ({ text, variant = 'default' }) => (
  <span className={`badge badge-${variant}`}>
    {text}
  </span>
);

export default Badge;
