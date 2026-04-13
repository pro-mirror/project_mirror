import React, { useEffect, useRef } from 'react';
import { View, StyleSheet, Animated } from 'react-native';
import { theme } from '../theme';

interface MirrorOrbProps {
  isActive?: boolean;
  size?: number;
}

export default function MirrorOrb({ isActive = false, size = 120 }: MirrorOrbProps) {
  const scale = useRef(new Animated.Value(1)).current;
  const opacity = useRef(new Animated.Value(0.8)).current;

  const orbSize = size * 0.67; // Main orb is ~67% of container
  const coreSize = size * 0.33; // Core is ~33% of container

  useEffect(() => {
    // Breathing animation
    Animated.loop(
      Animated.sequence([
        Animated.timing(scale, {
          toValue: 1.1,
          duration: 2000,
          useNativeDriver: true,
        }),
        Animated.timing(scale, {
          toValue: 1,
          duration: 2000,
          useNativeDriver: true,
        }),
      ])
    ).start();

    Animated.loop(
      Animated.sequence([
        Animated.timing(opacity, {
          toValue: 1,
          duration: 2000,
          useNativeDriver: true,
        }),
        Animated.timing(opacity, {
          toValue: 0.6,
          duration: 2000,
          useNativeDriver: true,
        }),
      ])
    ).start();
  }, []);

  return (
    <View style={[styles.container, { width: size, height: size }]}>
      {/* Outer glow */}
      <Animated.View
        style={[
          styles.outerGlow,
          {
            width: size,
            height: size,
            borderRadius: size / 2,
            transform: [{ scale }],
            opacity,
            backgroundColor: isActive ? theme.colors.accent : theme.colors.accent,
          },
        ]}
      />
      
      {/* Main orb */}
      <Animated.View
        style={[
          styles.orb,
          {
            width: orbSize,
            height: orbSize,
            borderRadius: orbSize / 2,
            transform: [{ scale: isActive ? 1.2 : 1 }],
            opacity,
            backgroundColor: theme.colors.accent,
          },
        ]}
      />
      
      {/* Inner core */}
      <View style={[styles.core, { width: coreSize, height: coreSize, borderRadius: coreSize / 2 }]} />
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    justifyContent: 'center',
    alignItems: 'center',
  },
  outerGlow: {
    position: 'absolute',
    opacity: 0.2,
  },
  orb: {
    position: 'absolute',
    shadowColor: theme.colors.accent,
    shadowOffset: { width: 0, height: 0 },
    shadowOpacity: 0.8,
    shadowRadius: 20,
  },
  core: {
    backgroundColor: '#fff',
    opacity: 0.9,
  },
});
