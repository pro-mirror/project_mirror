import React, { useState, useMemo } from 'react';
import { View, Text, ScrollView, StyleSheet, TouchableOpacity, Modal, RefreshControl } from 'react-native';
import { useQuery } from '@tanstack/react-query';
import { Ionicons } from '@expo/vector-icons';
import { episodesApi } from '../api/client';
import { theme } from '../theme';

interface Episode {
  id: string;
  text: string;
  reply_text?: string;
  timestamp: number;
  emotion_type?: string;
}

interface GroupedEpisodes {
  [date: string]: Episode[];
}

export default function ChronicleScreen() {
  const [selectedDate, setSelectedDate] = useState<string | null>(null);

  const { data: episodes, isLoading, refetch, isRefetching } = useQuery({
    queryKey: ['episodes'],
    queryFn: episodesApi.getEpisodes,
    staleTime: 5 * 60 * 1000, // 5分間はキャッシュを使用
  });

  // Group episodes by date
  const groupedEpisodes: GroupedEpisodes = useMemo(() => {
    if (!episodes) return {};
    
    return episodes.reduce((groups: GroupedEpisodes, episode: Episode) => {
      const date = new Date(episode.timestamp * 1000);
      const dateKey = date.toLocaleDateString('ja-JP', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
      });
      
      if (!groups[dateKey]) {
        groups[dateKey] = [];
      }
      groups[dateKey].push(episode);
      return groups;
    }, {});
  }, [episodes]);

  // Sort dates (newest first)
  const sortedDates = useMemo(() => {
    return Object.keys(groupedEpisodes).sort((a, b) => {
      const dateA = new Date(a.split('/').reverse().join('-'));
      const dateB = new Date(b.split('/').reverse().join('-'));
      return dateB.getTime() - dateA.getTime();
    });
  }, [groupedEpisodes]);

  const formatTime = (timestamp: number) => {
    const date = new Date(timestamp * 1000);
    return date.toLocaleTimeString('ja-JP', {
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const getEmotionIcon = (emotion?: string) => {
    switch (emotion) {
      case 'positive':
        return { name: 'happy-outline' as const, color: '#10b981' };
      case 'negative':
        return { name: 'sad-outline' as const, color: '#ef4444' };
      case 'neutral':
      default:
        return { name: 'remove-circle-outline' as const, color: '#6b7280' };
    }
  };

  const selectedEpisodes = selectedDate ? groupedEpisodes[selectedDate] : [];

  return (
    <View style={styles.container}>
      <ScrollView 
        style={styles.scrollView}
        refreshControl={
          <RefreshControl
            refreshing={isRefetching}
            onRefresh={refetch}
            tintColor={theme.colors.accent}
          />
        }
      >
        {isLoading && (
          <View style={styles.loadingContainer}>
            <Text style={styles.loadingText}>読み込み中...</Text>
          </View>
        )}

        {sortedDates.length > 0 && sortedDates.map((date) => {
          const episodesForDate = groupedEpisodes[date];
          const emotionCounts = episodesForDate.reduce((acc, ep) => {
            const emotion = ep.emotion_type || 'neutral';
            acc[emotion] = (acc[emotion] || 0) + 1;
            return acc;
          }, {} as Record<string, number>);

          return (
            <TouchableOpacity
              key={date}
              style={styles.dateCard}
              onPress={() => setSelectedDate(date)}
            >
              <View style={styles.dateCardHeader}>
                <Text style={styles.dateText}>{date}</Text>
                <View style={styles.dateCardBadge}>
                  <Text style={styles.dateCardBadgeText}>{episodesForDate.length}件</Text>
                </View>
              </View>
              <View style={styles.dateCardEmotions}>
                {emotionCounts.positive > 0 && (
                  <View style={styles.emotionChip}>
                    <Ionicons name="happy-outline" size={14} color="#10b981" />
                    <Text style={styles.emotionChipText}>{emotionCounts.positive}</Text>
                  </View>
                )}
                {emotionCounts.neutral > 0 && (
                  <View style={styles.emotionChip}>
                    <Ionicons name="remove-circle-outline" size={14} color="#6b7280" />
                    <Text style={styles.emotionChipText}>{emotionCounts.neutral}</Text>
                  </View>
                )}
                {emotionCounts.negative > 0 && (
                  <View style={styles.emotionChip}>
                    <Ionicons name="sad-outline" size={14} color="#ef4444" />
                    <Text style={styles.emotionChipText}>{emotionCounts.negative}</Text>
                  </View>
                )}
              </View>
            </TouchableOpacity>
          );
        })}
        
        {episodes && episodes.length === 0 && (
          <View style={styles.emptyState}>
            <Text style={styles.emptyText}>まだ記憶がありません</Text>
            <Text style={styles.emptySubtext}>
              対話を重ねることで、少しずつ積み重なっていきます
            </Text>
          </View>
        )}
      </ScrollView>

      {/* Episode List Modal */}
      <Modal
        visible={selectedDate !== null}
        animationType="slide"
        transparent={true}
        onRequestClose={() => setSelectedDate(null)}
      >
        <View style={styles.modalOverlay}>
          <View style={styles.modalContent}>
            <View style={styles.modalHeader}>
              <Text style={styles.modalTitle}>{selectedDate}</Text>
              <TouchableOpacity onPress={() => setSelectedDate(null)}>
                <Ionicons name="close" size={28} color={theme.colors.text} />
              </TouchableOpacity>
            </View>

            <ScrollView style={styles.modalBody}>
              {selectedEpisodes
                .sort((a, b) => a.timestamp - b.timestamp)
                .map((episode, index) => {
                  const icon = getEmotionIcon(episode.emotion_type);
                  return (
                    <View key={episode.id} style={styles.episodeItem}>
                      <View style={styles.episodeHeader}>
                        <Text style={styles.episodeTime}>{formatTime(episode.timestamp)}</Text>
                        <Ionicons name={icon.name} size={16} color={icon.color} />
                      </View>
                      
                      {/* User message */}
                      <View style={styles.userMessage}>
                        <Text style={styles.messageLabel}>あなた</Text>
                        <Text style={styles.episodeText}>{episode.text}</Text>
                      </View>
                      
                      {/* AI response */}
                      {episode.reply_text && (
                        <View style={styles.aiMessage}>
                          <Text style={styles.messageLabel}>Mirror</Text>
                          <Text style={styles.episodeText}>{episode.reply_text}</Text>
                        </View>
                      )}
                      
                      {index < selectedEpisodes.length - 1 && <View style={styles.episodeDivider} />}
                    </View>
                  );
                })}
            </ScrollView>
          </View>
        </View>
      </Modal>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: theme.colors.background,
  },
  scrollView: {
    flex: 1,
  },
  dateCard: {
    backgroundColor: theme.colors.backgroundAlt,
    margin: theme.spacing.md,
    padding: theme.spacing.lg,
    borderRadius: theme.borderRadius.md,
    borderLeftWidth: 4,
    borderLeftColor: theme.colors.accent,
  },
  dateCardHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: theme.spacing.md,
  },
  dateText: {
    fontSize: theme.fontSize.lg,
    color: theme.colors.text,
    fontWeight: '600',
  },
  dateCardBadge: {
    backgroundColor: theme.colors.accent + '20',
    paddingHorizontal: theme.spacing.md,
    paddingVertical: theme.spacing.xs,
    borderRadius: theme.borderRadius.sm,
  },
  dateCardBadgeText: {
    fontSize: theme.fontSize.sm,
    color: theme.colors.accent,
    fontWeight: '600',
  },
  dateCardEmotions: {
    flexDirection: 'row',
    gap: theme.spacing.sm,
  },
  emotionChip: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: theme.spacing.xs,
    backgroundColor: theme.colors.background,
    paddingHorizontal: theme.spacing.sm,
    paddingVertical: theme.spacing.xs,
    borderRadius: theme.borderRadius.sm,
  },
  emotionChipText: {
    fontSize: theme.fontSize.xs,
    color: theme.colors.textSecondary,
  },
  loadingContainer: {
    padding: theme.spacing.xl,
    alignItems: 'center',
  },
  loadingText: {
    color: theme.colors.textSecondary,
    fontSize: theme.fontSize.md,
  },
  emptyState: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    padding: theme.spacing.xxl,
    marginTop: 100,
  },
  emptyText: {
    fontSize: theme.fontSize.lg,
    color: theme.colors.textSecondary,
    marginBottom: theme.spacing.sm,
  },
  emptySubtext: {
    fontSize: theme.fontSize.sm,
    color: theme.colors.textSecondary,
    textAlign: 'center',
    lineHeight: 20,
  },
  modalOverlay: {
    flex: 1,
    backgroundColor: 'rgba(0, 0, 0, 0.7)',
    justifyContent: 'flex-end',
  },
  modalContent: {
    backgroundColor: theme.colors.background,
    borderTopLeftRadius: theme.borderRadius.lg,
    borderTopRightRadius: theme.borderRadius.lg,
    maxHeight: '85%',
  },
  modalHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: theme.spacing.lg,
    borderBottomWidth: 1,
    borderBottomColor: theme.colors.border,
  },
  modalTitle: {
    fontSize: theme.fontSize.xl,
    color: theme.colors.text,
    fontWeight: '600',
  },
  modalBody: {
    padding: theme.spacing.lg,
  },
  episodeItem: {
    marginBottom: theme.spacing.lg,
  },
  episodeHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: theme.spacing.sm,
  },
  episodeTime: {
    fontSize: theme.fontSize.sm,
    color: theme.colors.textSecondary,
  },
  userMessage: {
    marginBottom: theme.spacing.md,
  },
  aiMessage: {
    backgroundColor: theme.colors.backgroundAlt,
    padding: theme.spacing.md,
    borderRadius: theme.borderRadius.md,
    borderLeftWidth: 3,
    borderLeftColor: theme.colors.accent,
  },
  messageLabel: {
    fontSize: theme.fontSize.xs,
    color: theme.colors.textSecondary,
    marginBottom: theme.spacing.xs,
    fontWeight: '600',
  },
  episodeText: {
    fontSize: theme.fontSize.md,
    color: theme.colors.text,
    lineHeight: 24,
  },
  episodeDivider: {
    height: 1,
    backgroundColor: theme.colors.border,
    marginTop: theme.spacing.lg,
  },
});
