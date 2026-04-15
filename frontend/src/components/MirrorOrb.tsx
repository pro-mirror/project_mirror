import React, { useEffect, useRef } from 'react';
import { View, StyleSheet, Animated } from 'react-native';
import { theme } from '../theme';

interface MirrorOrbProps {
  isActive?: boolean;
  size?: number;
  mode?: 'orb' | 'waveform';
  audioLevel?: number;
}

export default function MirrorOrb({ 
  isActive = false, 
  size = 120, 
  mode = 'orb',
  audioLevel = 0 
}: MirrorOrbProps) {
  const scale = useRef(new Animated.Value(1)).current;
  const opacity = useRef(new Animated.Value(0.8)).current;

  // Waveform bars
  const bar1 = useRef(new Animated.Value(0.3)).current;
  const bar2 = useRef(new Animated.Value(0.5)).current;
  const bar3 = useRef(new Animated.Value(0.7)).current;
  const bar4 = useRef(new Animated.Value(0.5)).current;
  const bar5 = useRef(new Animated.Value(0.3)).current;

  const orbSize = size * 0.67; // Main orb is ~67% of container
  const coreSize = size * 0.33; // Core is ~33% of container

  useEffect(() => {
    if (mode === 'orb') {
      // Breathing animation for orb mode
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
    } else {
      // Waveform animation for voice mode
      const createBarAnimation = (bar: Animated.Value, delay: number) => {
        return Animated.loop(
          Animated.sequence([
            Animated.timing(bar, {
              toValue: 0.9,
              duration: 300 + delay,
              useNativeDriver: true,
            }),
            Animated.timing(bar, {
              toValue: 0.2,
              duration: 300 + delay,
              useNativeDriver: true,
            }),
          ])
        );
      };

      createBarAnimation(bar1, 0).start();
      createBarAnimation(bar2, 50).start();
      createBarAnimation(bar3, 100).start();
      createBarAnimation(bar4, 50).start();
      createBarAnimation(bar5, 0).start();
    }
  }, [mode]);

  if (mode === 'waveform') {
    // Voice waveform visualization
    return (
      <View style={[styles.container, { width: size, height: size }]}>
        <View style={styles.waveformContainer}>
          {[bar1, bar2, bar3, bar4, bar5].map((bar, index) => (
            <Animated.View
              key={index}
              style={[
                styles.waveformBar,
                {
                  height: size * 0.8,
                  width: size * 0.08,
                  transform: [{ scaleY: bar }],
                  backgroundColor: theme.colors.accent,
                },
              ]}
            />
          ))}
        </View>
      </View>
    );
  }

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
  waveformContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-evenly',
    width: '100%',
    height: '100%',
  },
  waveformBar: {
    borderRadius: 4,
    shadowColor: theme.colors.accent,
    shadowOffset: { width: 0, height: 0 },
    shadowOpacity: 0.8,
    shadowRadius: 10,
  },
});
