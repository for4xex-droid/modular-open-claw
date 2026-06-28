/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { motion } from 'framer-motion';

interface AiomeSkeletonProps {
  width?: string | number;
  height?: string | number;
  borderRadius?: string | number;
  className?: string;
  style?: React.CSSProperties;
}

export const AiomeSkeleton: React.FC<AiomeSkeletonProps> = ({ 
  width = '100%', 
  height = '1rem', 
  borderRadius = 'var(--radius-sm)', 
  className = '',
  style = {}
}) => {
  return (
    <motion.div
      className={`aiome-skeleton ${className}`}
      style={{
        width,
        height,
        borderRadius,
        background: 'var(--white-05)',
        overflow: 'hidden',
        position: 'relative',
        ...style
      }}
      animate={{ opacity: [0.4, 0.8, 0.4] }}
      transition={{ duration: 2, repeat: Infinity, ease: 'easeInOut' }}
    >
      <motion.div
        style={{
          position: 'absolute',
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          background: 'linear-gradient(90deg, transparent, var(--white-10), transparent)',
          width: '200%',
        }}
        animate={{ x: ['-100%', '50%'] }}
        transition={{ duration: 2, repeat: Infinity, ease: 'easeInOut' }}
      />
    </motion.div>
  );
};
