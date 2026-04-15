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
import { Audio } from 'expo-av';

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
  const [isRecording, setIsRecording] = useState(false);
  const [recording, setRecording] = useState<Audio.Recording | null>(null);
  const scrollViewRef = useRef<ScrollView>(null);
  const queryClient = useQueryClient();
  const hapticsInterval = useRef<NodeJS.Timeout | null>(null);

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
      
      // Invalidate graph cache so Constellation screen shows new data
      queryClient.invalidateQueries({ queryKey: ['core-value-graph'] });
      
      // Success haptic feedback
      Haptics.notificationAsync(Haptics.NotificationFeedbackType.Success);
    },
  });

  const voiceMutation = useMutation({
    mutationFn: chatApi.sendVoiceMessage,
    onSuccess: (data) => {
      // Add user's transcribed message if available
      if (data.transcribed_text) {
        const userMessage: Message = {
          id: Date.now().toString(),
          text: data.transcribed_text,
          sender: 'user',
          timestamp: new Date(),
        };
        setMessages((prev) => [...prev, userMessage]);
      }
      
      // Add mirror's response
      const mirrorMessage: Message = {
        id: (Date.now() + 1).toString(),
        text: data.reply_text,
        sender: 'mirror',
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, mirrorMessage]);
      
      // Invalidate caches
      queryClient.invalidateQueries({ queryKey: ['episodes'] });
      queryClient.invalidateQueries({ queryKey: ['core-value-graph'] });
      
      // Success haptic feedback (double pulse - heart beat)
      Haptics.notificationAsync(Haptics.NotificationFeedbackType.Success);
    },
    onError: (error) => {
      console.error('Failed to send voice message:', error);
      // Show error message
      const errorMessage: Message = {
        id: Date.now().toString(),
        text: '音声の送信に失敗しました。テキストで入力してみてください。',
        sender: 'mirror',
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, errorMessage]);
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

  // Cleanup haptics on unmount
  useEffect(() => {
    return () => {
      if (hapticsInterval.current) {
        clearInterval(hapticsInterval.current);
      }
    };
  }, []);

  const startRecording = async () => {
    try {
      // Request permissions
      const permission = await Audio.requestPermissionsAsync();
      if (!permission.granted) {
        console.log('Permission to access audio denied');
        return;
      }

      // Set audio mode
      await Audio.setAudioModeAsync({
        allowsRecordingIOS: true,
        playsInSilentModeIOS: true,
      });

      // Start recording
      const { recording: newRecording } = await Audio.Recording.createAsync(
        Audio.RecordingOptionsPresets.HIGH_QUALITY
      );
      setRecording(newRecording);
      setIsRecording(true);

      // Start haptic feedback (gentle pulse every 1.5 seconds)
      hapticsInterval.current = setInterval(() => {
        Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Light);
      }, 1500);

    } catch (error) {
      console.error('Failed to start recording:', error);
    }
  };

  const stopRecording = async () => {
    if (!recording) return;

    try {
      // Stop haptic feedback
      if (hapticsInterval.current) {
        clearInterval(hapticsInterval.current);
        hapticsInterval.current = null;
      }

      // Stop recording
      await recording.stopAndUnloadAsync();
      await Audio.setAudioModeAsync({
        allowsRecordingIOS: false,
      });

      const uri = recording.getURI();
      setRecording(null);
      setIsRecording(false);

      // Send audio to backend
      if (uri) {
        console.log('Recording saved to:', uri);
        
        // Send to API (response will add user message with transcribed text)
        voiceMutation.mutate({
          user_id: 'user-001', // TODO: Get from auth
          audio_uri: uri,
        });
      }
    } catch (error) {
      console.error('Failed to stop recording:', error);
      setIsRecording(false);
    }
  };

  const toggleRecording = () => {
    if (isRecording) {
      stopRecording();
    } else {
      startRecording();
    }
  };

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
      {/* Voice mode overlay */}
      {isRecording && (
        <View style={styles.voiceModeOverlay}>
          <View style={styles.voiceModeContent}>
            <MirrorOrb mode="waveform" size={200} />
            <Text style={styles.voiceModeText}>お話しください...</Text>
            <TouchableOpacity 
              style={styles.stopButton}
              onPress={stopRecording}
            >
              <Ionicons name="stop-circle" size={64} color={theme.colors.accent} />
            </TouchableOpacity>
          </View>
        </View>
      )}

      {/* Header with orb */}
      <View style={styles.headerRow}>
        <View style={styles.orbCorner}>
          <MirrorOrb 
            isActive={chatMutation.isPending || voiceMutation.isPending} 
            mode={isRecording ? 'waveform' : 'orb'}
            size={36} 
          />
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
        {(chatMutation.isPending || voiceMutation.isPending) && (
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
          editable={!isRecording}
        />
        <TouchableOpacity
          style={[styles.micButton, isRecording && styles.micButtonActive]}
          onPress={toggleRecording}
        >
          <Ionicons 
            name={isRecording ? "stop" : "mic"} 
            size={28} 
            color="#fff" 
          />
        </TouchableOpacity>
        <TouchableOpacity
          style={styles.sendButton}
          onPress={handleSend}
          disabled={chatMutation.isPending || voiceMutation.isPending || isRecording}
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
    alignItems: 'center',
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
  micButton: {
    backgroundColor: '#6366F1',
    width: 56,
    height: 56,
    borderRadius: 28,
    justifyContent: 'center',
    alignItems: 'center',
    marginRight: theme.spacing.sm,
  },
  micButtonActive: {
    backgroundColor: '#EF4444',
  },
  sendButton: {
    backgroundColor: theme.colors.accent,
    width: 48,
    height: 48,
    borderRadius: 24,
    justifyContent: 'center',
    alignItems: 'center',
  },
  voiceModeOverlay: {
    position: 'absolute',
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    backgroundColor: 'rgba(15, 23, 42, 0.95)',
    zIndex: 1000,
    justifyContent: 'center',
    alignItems: 'center',
  },
  voiceModeContent: {
    alignItems: 'center',
  },
  voiceModeText: {
    color: theme.colors.text,
    fontSize: theme.fontSize.xl,
    marginTop: theme.spacing.xl,
    marginBottom: theme.spacing.xl,
  },
  stopButton: {
    marginTop: theme.spacing.lg,
  },
});
