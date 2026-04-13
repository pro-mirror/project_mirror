import React from 'react';
import { View, Text, StyleSheet, ActivityIndicator } from 'react-native';
import { useQuery } from '@tanstack/react-query';
import Svg, { Circle, Line, Text as SvgText } from 'react-native-svg';

import { insightsApi } from '../api/client';
import { theme } from '../theme';

export default function ConstellationScreen() {
  const { data, isLoading, error } = useQuery({
    queryKey: ['graph'],
    queryFn: insightsApi.getGraph,
  });

  if (isLoading) {
    return (
      <View style={styles.container}>
        <ActivityIndicator color={theme.colors.accent} size="large" />
        <Text style={styles.loadingText}>読み込み中...</Text>
      </View>
    );
  }

  if (error) {
    return (
      <View style={styles.container}>
        <Text style={styles.errorText}>データの取得に失敗しました</Text>
      </View>
    );
  }

  // Simple layout for now - TODO: Implement force-directed graph
  const width = 350;
  const height = 500;
  const centerX = width / 2;
  const centerY = height / 2;

  return (
    <View style={styles.container}>
      <View style={styles.graphContainer}>
        <Svg width={width} height={height} style={styles.graph}>
        {/* Draw edges */}
        {data?.edges.map((edge, index) => {
          const sourceNode = data.nodes.find((n) => n.id === edge.source);
          const targetNode = data.nodes.find((n) => n.id === edge.target);
          
          // Simple circular layout
          const sourceIndex = data.nodes.findIndex((n) => n.id === edge.source);
          const targetIndex = data.nodes.findIndex((n) => n.id === edge.target);
          
          const sourceAngle = (sourceIndex / data.nodes.length) * 2 * Math.PI;
          const targetAngle = (targetIndex / data.nodes.length) * 2 * Math.PI;
          
          const radius = 150;
          const x1 = centerX + radius * Math.cos(sourceAngle);
          const y1 = centerY + radius * Math.sin(sourceAngle);
          const x2 = centerX + radius * Math.cos(targetAngle);
          const y2 = centerY + radius * Math.sin(targetAngle);
          
          return (
            <Line
              key={`edge-${index}`}
              x1={x1}
              y1={y1}
              x2={x2}
              y2={y2}
              stroke={edge.relation === 'FELT_GRATITUDE' ? theme.colors.accent : theme.colors.border}
              strokeWidth={edge.relation === 'FELT_GRATITUDE' ? 3 : 1}
              opacity={0.6}
            />
          );
        })}
        
        {/* Draw nodes */}
        {data?.nodes.map((node, index) => {
          const angle = (index / data.nodes.length) * 2 * Math.PI;
          const radius = 150;
          const x = centerX + radius * Math.cos(angle);
          const y = centerY + radius * Math.sin(angle);
          
          return (
            <React.Fragment key={node.id}>
              <Circle
                cx={x}
                cy={y}
                r={node.node_type === 'User' ? 20 : 15}
                fill={
                  node.node_type === 'User'
                    ? theme.colors.accent
                    : node.node_type === 'Person'
                    ? '#60A5FA'
                    : '#94A3B8'
                }
                opacity={0.8}
              />
              <SvgText
                x={x}
                y={y + 35}
                fontSize={12}
                fill={theme.colors.text}
                textAnchor="middle"
              >
                {node.label}
              </SvgText>
            </React.Fragment>
          );
        })}
      </Svg>
      </View>
      
      <Text style={styles.legend}>
        大切な記憶とのつながり
      </Text>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: theme.colors.background,
    padding: theme.spacing.lg,
  },
  graphContainer: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
  },
  graph: {
    marginVertical: theme.spacing.lg,
  },
  legend: {
    fontSize: theme.fontSize.xs,
    color: theme.colors.textSecondary,
    paddingBottom: theme.spacing.lg,
    textAlign: 'center',
  },
  loadingText: {
    color: theme.colors.textSecondary,
    marginTop: theme.spacing.md,
  },
  errorText: {
    color: theme.colors.accentDanger,
    fontSize: theme.fontSize.md,
  },
});
