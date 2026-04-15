import React, { useState, useRef } from 'react';
import { View, Text, StyleSheet, ActivityIndicator, TouchableOpacity, Modal, ScrollView, Animated } from 'react-native';
import { useQuery } from '@tanstack/react-query';
import Svg, { Circle, Line, Text as SvgText, G } from 'react-native-svg';
import { GestureDetector, Gesture } from 'react-native-gesture-handler';

import { insightsApi, episodesApi, GraphNode, EpisodeDetail, CoreValueDetail } from '../api/client';
import { theme } from '../theme';

export default function ConstellationScreen() {
  const [selectedEpisode, setSelectedEpisode] = useState<string | null>(null);
  const [selectedCoreValue, setSelectedCoreValue] = useState<string | null>(null);
  const fadeAnim = useState(new Animated.Value(0))[0];
  
  // Pan and zoom state
  const scale = useState(new Animated.Value(1))[0];
  const translateX = useState(new Animated.Value(0))[0];
  const translateY = useState(new Animated.Value(0))[0];
  const [baseScale, setBaseScale] = useState(1);
  const [baseTranslateX, setBaseTranslateX] = useState(0);
  const [baseTranslateY, setBaseTranslateY] = useState(0);
  
  // Custom node positions (for manual adjustments)
  const [customPositions, setCustomPositions] = useState<Map<string, { x: number; y: number }>>(new Map());
  
  // Track drag start positions per node
  const dragStartPositions = useRef<Map<string, { x: number; y: number }>>(new Map());

  const { data, isLoading, error } = useQuery({
    queryKey: ['core-value-graph'],
    queryFn: insightsApi.getCoreValueGraph,
    staleTime: 0, // Always consider data stale
    refetchOnMount: true, // Refetch when component mounts
    refetchOnWindowFocus: true, // Refetch when window/tab gains focus
  });

  React.useEffect(() => {
    if (error) {
      console.error('Graph error:', error);
    }
  }, [error]);

  const { data: episodeData, isLoading: isLoadingEpisode, error: episodeError } = useQuery({
    queryKey: ['episode', selectedEpisode],
    queryFn: () => episodesApi.getEpisodeByParentId(selectedEpisode!),
    enabled: !!selectedEpisode,
    retry: false, // Don't retry on 404
  });

  const { data: coreValueData, isLoading: isLoadingCoreValue, error: coreValueError } = useQuery({
    queryKey: ['core-value', selectedCoreValue],
    queryFn: () => insightsApi.getCoreValueDetail(selectedCoreValue!),
    enabled: !!selectedCoreValue,
    retry: false,
  });

  // Show modal with fade animation
  const showModal = () => {
    fadeAnim.setValue(0);
    Animated.timing(fadeAnim, {
      toValue: 1,
      duration: 300,
      useNativeDriver: true,
    }).start();
  };

  React.useEffect(() => {
    if (selectedEpisode || selectedCoreValue) {
      showModal();
    }
  }, [selectedEpisode, selectedCoreValue]);

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
        <Text style={styles.loadingText}>{String(error)}</Text>
      </View>
    );
  }

  if (!data || data.nodes.length === 0) {
    return (
      <View style={styles.container}>
        <Text style={styles.errorText}>データがありません</Text>
        <Text style={styles.loadingText}>会話を重ねると、ここにグラフが表示されます</Text>
      </View>
    );
  }

  // Calculate positions for nodes
  const width = 300; // Further reduced
  const height = 440;
  const centerX = width / 2;
  const centerY = height / 2;
  
  // Helper function to clamp position within bounds
  const clampPosition = (pos: { x: number; y: number }) => {
    const margin = 20; // Keep nodes at least 20px from edges
    return {
      x: Math.max(margin, Math.min(width - margin, pos.x)),
      y: Math.max(margin, Math.min(height - margin, pos.y)),
    };
  };

  const coreValues = data?.nodes.filter(n => n.node_type === 'CoreValue') || [];
  const episodes = data?.nodes.filter(n => n.node_type === 'Episode') || [];

  // Position CoreValues in a circle
  const cvPositions = new Map<string, { x: number; y: number }>();
  coreValues.forEach((cv, index) => {
    // Use custom position if available
    if (customPositions.has(cv.id)) {
      cvPositions.set(cv.id, customPositions.get(cv.id)!);
      return;
    }
    
    const angle = (index / coreValues.length) * 2 * Math.PI;
    // Dynamically adjust radius based on number of nodes to prevent overlap
    const minRadius = 90;
    const nodeSize = 40; // Max node size + padding
    const requiredCircumference = coreValues.length * nodeSize * 1.5; // 1.5x for spacing
    const requiredRadius = requiredCircumference / (2 * Math.PI);
    const radius = Math.max(minRadius, requiredRadius);
    
    cvPositions.set(cv.id, {
      x: centerX + radius * Math.cos(angle),
      y: centerY + radius * Math.sin(angle),
    });
  });

  // Position Episodes around their connected CoreValues
  const epPositions = new Map<string, { x: number; y: number }>();
  const epCounts = new Map<string, number>(); // Track how many episodes per CoreValue
  
  episodes.forEach((ep) => {
    // Use custom position if available
    if (customPositions.has(ep.id)) {
      epPositions.set(ep.id, customPositions.get(ep.id)!);
      return;
    }
    
    const connectedEdges = data?.edges.filter(e => e.source === ep.id) || [];
    if (connectedEdges.length > 0) {
      const cvId = connectedEdges[0].target;
      const cvPos = cvPositions.get(cvId);
      if (cvPos) {
        // Count episodes for this CoreValue
        const count = epCounts.get(cvId) || 0;
        const totalEpisodes = episodes.filter(e => 
          data?.edges.some(edge => edge.source === e.id && edge.target === cvId)
        ).length;
        epCounts.set(cvId, count + 1);
        
        // Dynamically adjust distance based on number of episodes
        const episodeSize = 28;
        const minDistance = 45;
        const requiredCircumference = totalEpisodes * episodeSize * 1.5;
        const requiredRadius = requiredCircumference / (2 * Math.PI);
        const distance = Math.max(minDistance, requiredRadius);
        
        const angleOffset = (count / Math.max(1, totalEpisodes)) * Math.PI * 2;
        epPositions.set(ep.id, {
          x: cvPos.x + distance * Math.cos(angleOffset),
          y: cvPos.y + distance * Math.sin(angleOffset),
        });
      }
    }
  });

  const handleNodePress = (node: GraphNode) => {
    if (node.node_type === 'Episode' && node.parent_id) {
      setSelectedEpisode(node.parent_id);
      setSelectedCoreValue(null);
    } else if (node.node_type === 'CoreValue') {
      setSelectedCoreValue(node.label);
      setSelectedEpisode(null);
    }
  };

  const closeModal = () => {
    setSelectedEpisode(null);
    setSelectedCoreValue(null);
  };

  // Pinch gesture for zoom
  const pinchGesture = Gesture.Pinch()
    .onUpdate((e) => {
      const newScale = Math.max(0.8, Math.min(3, baseScale * e.scale));
      scale.setValue(newScale);
      
      // Adjust translation based on focal point
      const focalX = e.focalX;
      const focalY = e.focalY;
      const deltaScale = newScale / baseScale;
      
      translateX.setValue(focalX + (baseTranslateX - focalX) * deltaScale);
      translateY.setValue(focalY + (baseTranslateY - focalY) * deltaScale);
    })
    .onEnd((e) => {
      const newScale = Math.max(0.8, Math.min(3, baseScale * e.scale));
      setBaseScale(newScale);
      
      const focalX = e.focalX;
      const focalY = e.focalY;
      const deltaScale = newScale / baseScale;
      
      setBaseTranslateX(focalX + (baseTranslateX - focalX) * deltaScale);
      setBaseTranslateY(focalY + (baseTranslateY - focalY) * deltaScale);
    });

  // Pan gesture for moving
  const panGesture = Gesture.Pan()
    .minDistance(10) // Require 10px movement to activate
    .onUpdate((e) => {
      translateX.setValue(baseTranslateX + e.translationX);
      translateY.setValue(baseTranslateY + e.translationY);
    })
    .onEnd((e) => {
      setBaseTranslateX(baseTranslateX + e.translationX);
      setBaseTranslateY(baseTranslateY + e.translationY);
    });

  const composedGesture = Gesture.Simultaneous(
    pinchGesture,
    panGesture
  );

  const formatDate = (timestamp: number) => {
    const date = new Date(timestamp * 1000);
    return `${date.getFullYear()}年${date.getMonth() + 1}月${date.getDate()}日`;
  };

  const formatTime = (timestamp: number) => {
    const date = new Date(timestamp * 1000);
    return `${date.getHours()}:${String(date.getMinutes()).padStart(2, '0')}`;
  };

  return (
    <View style={styles.container}>
      {/* Explanation */}
      {/* <View style={styles.explanationBox}> */}
        {/* <Text style={styles.explanationTitle}>あなたの記憶のつながり</Text> */}
       {/* <Text style={styles.explanationText}>
          大切にしている価値観とそれに関連するエピソードを可視化しています。
        </Text> */}
        {/* <Text style={styles.loadingText}>
          価値観:{coreValues.length}件 / エピソード:{episodes.length}件
        </Text>  */}
      {/* </View> */}

      {/* Legend */}
      <View style={styles.legendContainer}>
        <View style={styles.legendItem}>
          <View style={[styles.legendDot, { backgroundColor: theme.colors.accent }]} />
          <Text style={styles.legendText}>価値観</Text>
        </View>
        <View style={styles.legendItem}>
          <View style={[styles.legendDot, { backgroundColor: '#9333ea' }]} />
          <Text style={styles.legendText}>エピソード</Text>
        </View>
      </View>

      <View style={styles.graphContainer}>
        <GestureDetector gesture={composedGesture}>
          <Animated.View
            style={{
              transform: [
                { scale },
                { translateX },
                { translateY },
              ],
            }}
          >
            {/* Background layer with edges */}
            <View style={{ position: 'relative' }}>
              <Svg width={width} height={height}>
                <G>
                  {/* Draw edges */}
                  {data?.edges.map((edge, index) => {
                    const sourcePos = epPositions.get(edge.source);
                    const targetPos = cvPositions.get(edge.target);
                    
                    if (!sourcePos || !targetPos) return null;
                    
                    return (
                      <Line
                        key={`edge-${index}`}
                        x1={sourcePos.x}
                        y1={sourcePos.y}
                        x2={targetPos.x}
                        y2={targetPos.y}
                        stroke={theme.colors.accent}
                        strokeWidth={1}
                        opacity={0.6}
                      />
                    );
                  })}
                </G>
              </Svg>
              
              {/* Foreground layer with interactive nodes (overlay) */}
              <View
                style={{
                  position: 'absolute',
                  width: width,
                  height: height,
                  top: 0,
                  left: 0,
                }}
                pointerEvents="box-none"
              >
              {coreValues.map((node) => {
                const pos = cvPositions.get(node.id);
                if (!pos) return null;
                
                const radius = Math.min(15 + (node.total_weight || 0) * 3, 30); // Cap at 30px
                const size = radius * 2 + 10;
                
                // Create drag gesture for this node
                const nodeDrag = Gesture.Pan()
                  .maxPointers(1)
                  .minDistance(5)
                  .onBegin(() => {
                    // Save the current position at drag start
                    dragStartPositions.current.set(node.id, { x: pos.x, y: pos.y });
                  })
                  .onUpdate((e) => {
                    const startPos = dragStartPositions.current.get(node.id);
                    if (!startPos) return;
                    
                    const newPos = {
                      x: startPos.x + e.translationX * 0.8,
                      y: startPos.y + e.translationY * 0.8,
                    };
                    const clampedPos = clampPosition(newPos);
                    setCustomPositions(prev => new Map(prev).set(node.id, clampedPos));
                  })
                  .onFinalize(() => {
                    // Clean up drag start position
                    dragStartPositions.current.delete(node.id);
                  });
                
                return (
                  <GestureDetector key={`touch-${node.id}`} gesture={nodeDrag}>
                    <View pointerEvents="box-none">
                      <TouchableOpacity
                        style={{
                          position: 'absolute',
                          left: pos.x - size / 2,
                          top: pos.y - size / 2,
                          width: size,
                          height: size,
                          backgroundColor: theme.colors.accent,
                          borderRadius: size / 2,
                          opacity: 0.9,
                          justifyContent: 'center',
                          alignItems: 'center',
                        }}
                        activeOpacity={0.7}
                        onPress={() => handleNodePress(node)}
                      >
                        <Text style={{
                          color: '#ffffff',
                          fontSize: Math.max(7, radius / 3),
                          fontWeight: '700',
                          textAlign: 'center',
                          paddingHorizontal: 2,
                        }} numberOfLines={2}>
                          {node.label.length > 6 ? node.label.substring(0, 5) + '..' : node.label}
                      </Text>
                    </TouchableOpacity>
                    </View>
                  </GestureDetector>
                );
              })}
              
              {episodes.map((node) => {
                const pos = epPositions.get(node.id);
                if (!pos) return null;
                
                const size = 28;
                
                // Create drag gesture for this episode node
                const nodeDrag = Gesture.Pan()
                  .maxPointers(1)
                  .minDistance(5)
                  .onBegin(() => {
                    // Save the current position at drag start
                    dragStartPositions.current.set(node.id, { x: pos.x, y: pos.y });
                  })
                  .onUpdate((e) => {
                    const startPos = dragStartPositions.current.get(node.id);
                    if (!startPos) return;
                    
                    const newPos = {
                      x: startPos.x + e.translationX * 0.8,
                      y: startPos.y + e.translationY * 0.8,
                    };
                    const clampedPos = clampPosition(newPos);
                    setCustomPositions(prev => new Map(prev).set(node.id, clampedPos));
                  })
                  .onFinalize(() => {
                    // Clean up drag start position
                    dragStartPositions.current.delete(node.id);
                  });
                
                return (
                  <GestureDetector key={`touch-${node.id}`} gesture={nodeDrag}>
                    <View pointerEvents="box-none">
                      <TouchableOpacity
                        style={{
                          position: 'absolute',
                          left: pos.x - size / 2,
                          top: pos.y - size / 2,
                          width: size,
                          height: size,
                          backgroundColor: '#9333ea',
                          borderRadius: size / 2,
                          opacity: 0.85,
                          borderWidth: 2,
                          borderColor: theme.colors.background,
                        }}
                        activeOpacity={0.6}
                        onPress={() => handleNodePress(node)}
                      />
                    </View>
                  </GestureDetector>
                );
              })}
            </View>
          </View>
          </Animated.View>
        </GestureDetector>
      </View>
      
      <Text style={styles.legend}>
        ノードをドラッグして移動 / タップして詳細表示 / ピンチで拡大縮小
        {customPositions.size > 0 && (
          <>
            {' / '}
            <Text 
              onPress={() => setCustomPositions(new Map())}
              style={{ color: theme.colors.textSecondary }}
            >
              位置リセット
            </Text>
          </>
        )}
      </Text>

      {/* Episode Detail Modal */}
      <Modal
        visible={!!selectedEpisode}
        transparent
        animationType="none"
        onRequestClose={closeModal}
      >
        <Animated.View style={[styles.modalOverlay, { opacity: fadeAnim }]}>
          <Animated.View style={[styles.modalContent, {
            transform: [{
              scale: fadeAnim.interpolate({
                inputRange: [0, 1],
                outputRange: [0.9, 1],
              }),
            }],
          }]}>
            <View style={styles.modalHeader}>
              <Text style={styles.modalTitle}>
                {episodeData ? formatDate(episodeData.timestamp) : 'エピソード'}
              </Text>
              <TouchableOpacity onPress={closeModal}>
                <Text style={styles.closeButton}>✕</Text>
              </TouchableOpacity>
            </View>

            {episodeError ? (
              <View style={styles.loadingContainer}>
                <Text style={styles.errorText}>データが見つかりません</Text>
                <Text style={styles.loadingText}>このエピソードの会話データが存在しないか、削除された可能性があります。</Text>
              </View>
            ) : isLoadingEpisode || !episodeData ? (
              <View style={styles.loadingContainer}>
                <ActivityIndicator color={theme.colors.accent} size="large" />
                <Text style={styles.loadingText}>読み込み中...</Text>
              </View>
            ) : (
              <ScrollView style={styles.modalScroll}>
                {episodeData.core_values && episodeData.core_values.length > 0 && (
                  <View style={styles.metaSection}>
                    <Text style={styles.metaTitle}>💎 この会話で大切にしていたこと</Text>
                    {episodeData.core_values.map((cv, idx) => (
                      <Text key={idx} style={styles.metaValue}>• {cv}</Text>
                    ))}
                  </View>
                )}

                {episodeData.persons && episodeData.persons.length > 0 && (
                  <View style={styles.metaSection}>
                    <Text style={styles.metaTitle}>👤 登場人物</Text>
                    {episodeData.persons.map((person, idx) => (
                      <Text key={idx} style={styles.metaValue}>• {person}</Text>
                    ))}
                  </View>
                )}

                <View style={styles.conversationSection}>
                  <Text style={styles.metaTitle}>💬 会話の内容</Text>
                  {episodeData.messages && episodeData.messages.length > 0 ? (
                    episodeData.messages.map((msg, idx) => (
                      <View
                        key={idx}
                        style={[
                          styles.messageBubble,
                          msg.role === 'user' ? styles.userMessage : styles.assistantMessage,
                        ]}
                      >
                        <Text style={styles.messageRole}>
                          {msg.role === 'user' ? 'あなた' : 'Mirror'}
                        </Text>
                        <Text style={styles.messageContent}>{msg.content}</Text>
                        <Text style={styles.messageTime}>{formatTime(msg.timestamp)}</Text>
                      </View>
                    ))
                  ) : (
                    <Text style={styles.loadingText}>会話データがありません</Text>
                  )}
                </View>
              </ScrollView>
            )}
          </Animated.View>
        </Animated.View>
      </Modal>

      {/* CoreValue Detail Modal */}
      <Modal
        visible={!!selectedCoreValue}
        transparent
        animationType="none"
        onRequestClose={closeModal}
      >
        <Animated.View style={[styles.modalOverlay, { opacity: fadeAnim }]}>
          <Animated.View style={[styles.modalContent, {
            transform: [{
              scale: fadeAnim.interpolate({
                inputRange: [0, 1],
                outputRange: [0.9, 1],
              }),
            }],
          }]}>
            <View style={styles.modalHeader}>
              <Text style={styles.modalTitle}>{coreValueData?.value_name || '価値観'}</Text>
              <TouchableOpacity onPress={closeModal}>
                <Text style={styles.closeButton}>✕</Text>
              </TouchableOpacity>
            </View>

            {coreValueError ? (
              <View style={styles.loadingContainer}>
                <Text style={styles.errorText}>データが見つかりません</Text>
                <Text style={styles.loadingText}>この価値観のデータが存在しないか、削除された可能性があります。</Text>
              </View>
            ) : isLoadingCoreValue || !coreValueData ? (
              <View style={styles.loadingContainer}>
                <ActivityIndicator color={theme.colors.accent} size="large" />
                <Text style={styles.loadingText}>読み込み中...</Text>
              </View>
            ) : (
              <ScrollView style={styles.modalScroll}>
                <Text style={styles.weightText}>
                  重要度: {coreValueData.total_weight.toFixed(1)}
                </Text>

                <View style={styles.contextsSection}>
                  <Text style={styles.metaTitle}>この価値観が表れたエピソード</Text>
                  {coreValueData.contexts && coreValueData.contexts.length > 0 ? (
                    coreValueData.contexts.map((ctx, idx) => (
                      <TouchableOpacity
                        key={idx}
                        style={styles.contextCard}
                        onPress={() => {
                          setSelectedCoreValue(null);
                          setSelectedEpisode(ctx.episode_parent_id);
                        }}
                      >
                        <Text style={styles.contextDate}>
                          {formatDate(ctx.timestamp)}
                        </Text>
                        <Text style={styles.contextText}>{ctx.context}</Text>
                        <Text style={styles.contextWeight}>
                          重み: {ctx.weight.toFixed(1)}
                        </Text>
                      </TouchableOpacity>
                    ))
                  ) : (
                    <Text style={styles.loadingText}>関連するエピソードがありません</Text>
                  )}
                </View>
              </ScrollView>
            )}
          </Animated.View>
        </Animated.View>
      </Modal>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: theme.colors.background,
    padding: theme.spacing.lg,
  },
  explanationBox: {
    backgroundColor: theme.colors.backgroundAlt,
    padding: theme.spacing.md,
    borderRadius: theme.borderRadius.md,
    marginBottom: theme.spacing.md,
  },
  explanationTitle: {
    fontSize: theme.fontSize.lg,
    fontWeight: '600',
    color: theme.colors.text,
    marginBottom: theme.spacing.xs,
  },
  explanationText: {
    fontSize: theme.fontSize.sm,
    color: theme.colors.textSecondary,
    lineHeight: 20,
  },
  legendContainer: {
    flexDirection: 'row',
    justifyContent: 'center',
    gap: theme.spacing.md,
    marginBottom: theme.spacing.md,
  },
  legendItem: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: theme.spacing.xs,
  },
  legendDot: {
    width: 12,
    height: 12,
    borderRadius: 6,
  },
  legendText: {
    fontSize: theme.fontSize.xs,
    color: theme.colors.textSecondary,
  },
  graphContainer: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    paddingHorizontal: theme.spacing.md, // Increased padding
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
    textAlign: 'center',
    paddingHorizontal: theme.spacing.md,
  },
  loadingContainer: {
    padding: theme.spacing.xl,
    alignItems: 'center',
    justifyContent: 'center',
    width: '100%',
  },
  errorText: {
    color: theme.colors.accentDanger,
    fontSize: theme.fontSize.sm,
    textAlign: 'center',
    paddingHorizontal: theme.spacing.md,
  },
  // Modal styles
  modalOverlay: {
    flex: 1,
    backgroundColor: 'rgba(0, 0, 0, 0.7)',
    justifyContent: 'center',
    alignItems: 'center',
    padding: theme.spacing.lg,
  },
  modalContent: {
    backgroundColor: theme.colors.background,
    borderRadius: theme.borderRadius.lg,
    width: '100%',
    maxHeight: '80%',
    padding: theme.spacing.md,
  },
  modalHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: theme.spacing.md,
    paddingBottom: theme.spacing.sm,
    borderBottomWidth: 1,
    borderBottomColor: theme.colors.border,
  },
  modalTitle: {
    fontSize: theme.fontSize.xl,
    fontWeight: '700',
    color: theme.colors.text,
    flex: 1,
    marginRight: theme.spacing.sm,
  },
  closeButton: {
    fontSize: 24,
    color: theme.colors.textSecondary,
    paddingHorizontal: theme.spacing.sm,
  },
  modalScroll: {
    maxHeight: '100%',
  },
  metaSection: {
    marginBottom: theme.spacing.md,
    padding: theme.spacing.md,
    backgroundColor: theme.colors.backgroundAlt,
    borderRadius: theme.borderRadius.md,
  },
  metaTitle: {
    fontSize: theme.fontSize.md,
    fontWeight: '600',
    color: theme.colors.text,
    marginBottom: theme.spacing.sm,
  },
  metaValue: {
    fontSize: theme.fontSize.sm,
    color: theme.colors.textSecondary,
    marginLeft: theme.spacing.sm,
    marginTop: theme.spacing.xs,
  },
  conversationSection: {
    marginTop: theme.spacing.md,
  },
  messageBubble: {
    marginBottom: theme.spacing.md,
    padding: theme.spacing.md,
    borderRadius: theme.borderRadius.md,
  },
  userMessage: {
    backgroundColor: theme.colors.accent + '20',
    alignSelf: 'flex-start',
    maxWidth: '85%',
  },
  assistantMessage: {
    backgroundColor: theme.colors.backgroundAlt,
    alignSelf: 'flex-start',
    maxWidth: '85%',
  },
  messageRole: {
    fontSize: theme.fontSize.xs,
    fontWeight: '600',
    color: theme.colors.textSecondary,
    marginBottom: theme.spacing.xs,
  },
  messageContent: {
    fontSize: theme.fontSize.sm,
    color: theme.colors.text,
    lineHeight: 20,
  },
  messageTime: {
    fontSize: theme.fontSize.xs,
    color: theme.colors.textSecondary,
    marginTop: theme.spacing.xs,
    textAlign: 'right',
  },
  weightText: {
    fontSize: theme.fontSize.md,
    color: theme.colors.accent,
    fontWeight: '600',
    marginBottom: theme.spacing.md,
  },
  contextsSection: {
    marginTop: theme.spacing.md,
  },
  contextCard: {
    backgroundColor: theme.colors.backgroundAlt,
    padding: theme.spacing.md,
    borderRadius: theme.borderRadius.md,
    marginBottom: theme.spacing.sm,
    borderLeftWidth: 3,
    borderLeftColor: theme.colors.accent,
  },
  contextDate: {
    fontSize: theme.fontSize.xs,
    color: theme.colors.textSecondary,
    marginBottom: theme.spacing.xs,
  },
  contextText: {
    fontSize: theme.fontSize.sm,
    color: theme.colors.text,
    lineHeight: 20,
    marginBottom: theme.spacing.xs,
  },
  contextWeight: {
    fontSize: theme.fontSize.xs,
    color: theme.colors.accent,
    fontWeight: '600',
  },
});
