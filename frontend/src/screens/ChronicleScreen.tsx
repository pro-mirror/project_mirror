import React from 'react';
import { View, Text, ScrollView, StyleSheet, TouchableOpacity } from 'react-native';
import { theme } from '../theme';

interface Episode {
  id: string;
  date: string;
  title: string;
  summary: string;
}

// Mock data - TODO: Fetch from API
const mockEpisodes: Episode[] = [
  {
    id: '1',
    date: '2026-04-13',
    title: '妻の手料理への感謝',
    summary: '今日も美味しいご飯を作って待っててくれた。本当にありがたい。',
  },
  {
    id: '2',
    date: '2026-04-12',
    title: '田中さんの気遣い',
    summary: '仕事で大変な時に、田中さんが助けてくれた。',
  },
];

export default function ChronicleScreen() {
  return (
    <View style={styles.container}>
      <ScrollView style={styles.scrollView}>
        {mockEpisodes.map((episode) => (
          <TouchableOpacity key={episode.id} style={styles.card}>
            <View style={styles.cardHeader}>
              <Text style={styles.date}>{episode.date}</Text>
            </View>
            <Text style={styles.cardTitle}>{episode.title}</Text>
            <Text style={styles.cardSummary}>{episode.summary}</Text>
            <View style={styles.glowLine} />
          </TouchableOpacity>
        ))}
        
        {mockEpisodes.length === 0 && (
          <View style={styles.emptyState}>
            <Text style={styles.emptyText}>まだ記憶がありません</Text>
            <Text style={styles.emptySubtext}>
              対話を重ねることで、少しずつ積み重なっていきます
            </Text>
          </View>
        )}
      </ScrollView>
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
  card: {
    backgroundColor: theme.colors.backgroundAlt,
    margin: theme.spacing.md,
    padding: theme.spacing.lg,
    borderRadius: theme.borderRadius.md,
    borderLeftWidth: 3,
    borderLeftColor: theme.colors.accent,
  },
  cardHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    marginBottom: theme.spacing.sm,
  },
  date: {
    fontSize: theme.fontSize.xs,
    color: theme.colors.textSecondary,
  },
  cardTitle: {
    fontSize: theme.fontSize.lg,
    color: theme.colors.text,
    fontWeight: '600',
    marginBottom: theme.spacing.sm,
  },
  cardSummary: {
    fontSize: theme.fontSize.md,
    color: theme.colors.textSecondary,
    lineHeight: 22,
  },
  glowLine: {
    height: 1,
    backgroundColor: theme.colors.accent,
    marginTop: theme.spacing.md,
    opacity: 0.3,
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
});
