import React, { useState, useRef, useEffect } from 'react';
import {
  View,
  Text,
  TextInput,
  TouchableOpacity,
  ScrollView,
  StyleSheet,
  ActivityIndicator,
  KeyboardAvoidingView,
  Platform,
} from 'react-native';
import { Ionicons } from '@expo/vector-icons';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import * as Haptics from 'expo-haptics';

import { chatApi } from '../api/client';
import { theme } from '../theme';
import MirrorOrb from '../components/MirrorOrb';

interface Message {
  id: string;
  text: string;
  sender: 'user' | 'mirror';
  timestamp: Date;
}

export default function HomeScreen() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [inputText, setInputText] = useState('');
  const scrollViewRef = useRef<ScrollView>(null);
  const queryClient = useQueryClient();

  const chatMutation = useMutation({
    mutationFn: chatApi.sendMessage,
    onSuccess: (data) => {
      // Add mirror's response
      const mirrorMessage: Message = {
        id: Date.now().toString(),
        text: data.reply_text,
        sender: 'mirror',
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, mirrorMessage]);
      
      // Invalidate episodes cache so Chronicle screen shows new data
      queryClient.invalidateQueries({ queryKey: ['episodes'] });
      
      // Success haptic feedback
      Haptics.notificationAsync(Haptics.NotificationFeedbackType.Success);
    },
  });

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    if (messages.length > 0) {
      setTimeout(() => {
        scrollViewRef.current?.scrollToEnd({ animated: true });
      }, 100);
    }
  }, [messages]);

  const handleSend = () => {
    if (!inputText.trim()) return;

    // Add user message
    const userMessage: Message = {
      id: Date.now().toString(),
      text: inputText,
      sender: 'user',
      timestamp: new Date(),
    };
    setMessages((prev) => [...prev, userMessage]);

    // Send to API
    chatMutation.mutate({
      user_id: 'user-001', // TODO: Get from auth
      text: inputText,
    });

    setInputText('');
  };

  const handleReset = () => {
    setMessages([]);
    chatMutation.reset();
  };

  return (
    <KeyboardAvoidingView 
      style={styles.container}
      behavior={Platform.OS === 'ios' ? 'padding' : undefined}
      keyboardVerticalOffset={Platform.OS === 'ios' ? 90 : 0}
    >
      {/* Header with orb */}
      <View style={styles.headerRow}>
        <View style={styles.orbCorner}>
          <MirrorOrb isActive={chatMutation.isPending} size={36} />
        </View>
        <TouchableOpacity 
          style={styles.resetButton} 
          onPress={handleReset}
        >
          <Ionicons 
            name="refresh-outline" 
            size={24} 
            color={theme.colors.textSecondary} 
          />
        </TouchableOpacity>
      </View>

      {/* Messages */}
      <ScrollView 
        ref={scrollViewRef}
        style={styles.messagesContainer}
        contentContainerStyle={styles.messagesContent}
        keyboardShouldPersistTaps="handled"
        keyboardDismissMode="on-drag"
      >
        {/* Welcome message */}
        {messages.length === 0 && (
          <View style={styles.welcomeContainer}>
            <View style={styles.welcomeBubble}>
              <Text style={styles.welcomeText}>
                こんにちは。私はあなたのデジタルツイン（分身）です。鏡に向かって話すような感覚で、なんでも語ってくださいね。特にあなたが大切にしていることなどを教えていただけるとうれしいです。
              </Text>
            </View>
          </View>
        )}
        
        {messages.map((message) => (
          <View
            key={message.id}
            style={[
              styles.messageBubble,
              message.sender === 'user' ? styles.userBubble : styles.mirrorBubble,
            ]}
          >
            <Text style={styles.messageText}>{message.text}</Text>
          </View>
        ))}
        {chatMutation.isPending && (
          <View style={styles.loadingContainer}>
            <ActivityIndicator color={theme.colors.accent} />
            <Text style={styles.loadingText}>考え中...</Text>
          </View>
        )}
      </ScrollView>

      {/* Input Area */}
      <View style={styles.inputContainer}>
        <TextInput
          style={styles.input}
          value={inputText}
          onChangeText={setInputText}
          placeholder="お話しください..."
          placeholderTextColor={theme.colors.textSecondary}
          multiline
        />
        <TouchableOpacity
          style={styles.sendButton}
          onPress={handleSend}
          disabled={chatMutation.isPending}
        >
          <Ionicons name="send" size={24} color="#fff" />
        </TouchableOpacity>
      </View>
    </KeyboardAvoidingView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: theme.colors.background,
  },
  headerRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingHorizontal: theme.spacing.md,
    paddingVertical: theme.spacing.sm,
    backgroundColor: theme.colors.backgroundAlt,
    borderBottomWidth: 1,
    borderBottomColor: theme.colors.border,
  },
  orbCorner: {
    width: 36,
    height: 36,
  },
  resetButton: {
    width: 36,
    height: 36,
    justifyContent: 'center',
    alignItems: 'center',
  },
  messagesContainer: {
    flex: 1,
  },
  messagesContent: {
    padding: theme.spacing.md,
    paddingBottom: 200,
  },
  welcomeContainer: {
    alignItems: 'center',
    marginTop: theme.spacing.xl,
    marginBottom: theme.spacing.xl,
  },
  welcomeBubble: {
    backgroundColor: '#1a2642',
    padding: theme.spacing.lg,
    borderRadius: theme.borderRadius.md,
    borderLeftWidth: 3,
    borderLeftColor: theme.colors.accent,
    maxWidth: '90%',
  },
  welcomeText: {
    color: theme.colors.text,
    fontSize: theme.fontSize.md,
    lineHeight: 24,
    textAlign: 'left',
  },
  messageBubble: {
    padding: theme.spacing.md,
    borderRadius: theme.borderRadius.md,
    marginBottom: theme.spacing.md,
    maxWidth: '85%',
    flexShrink: 1,
  },
  userBubble: {
    backgroundColor: theme.colors.backgroundAlt,
    alignSelf: 'flex-end',
  },
  mirrorBubble: {
    backgroundColor: '#1a2642',
    alignSelf: 'flex-start',
    borderLeftWidth: 3,
    borderLeftColor: theme.colors.accent,
  },
  messageText: {
    color: theme.colors.text,
    fontSize: theme.fontSize.md,
    lineHeight: 24,
  },
  loadingContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    padding: theme.spacing.md,
  },
  loadingText: {
    color: theme.colors.textSecondary,
    marginLeft: theme.spacing.sm,
  },
  inputContainer: {
    flexDirection: 'row',
    padding: theme.spacing.md,
    backgroundColor: theme.colors.backgroundAlt,
    borderTopWidth: 1,
    borderTopColor: theme.colors.border,
  },
  input: {
    flex: 1,
    backgroundColor: theme.colors.background,
    color: theme.colors.text,
    padding: theme.spacing.md,
    borderRadius: theme.borderRadius.md,
    marginRight: theme.spacing.sm,
    fontSize: theme.fontSize.md,
  },
  sendButton: {
    backgroundColor: theme.colors.accent,
    width: 48,
    height: 48,
    borderRadius: 24,
    justifyContent: 'center',
    alignItems: 'center',
  },
});
