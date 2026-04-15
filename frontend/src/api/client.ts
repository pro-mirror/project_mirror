import axios from 'axios';

const API_BASE_URL = process.env.EXPO_PUBLIC_API_BASE_URL || 'http://localhost:8080/api/v1';

export const api = axios.create({
  baseURL: API_BASE_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});

export interface ChatMessage {
  user_id: string;
  text: string;
}

export interface ChatResponse {
  reply_text: string;
  emotion_detected: string;
}

export interface GraphNode {
  id: string;
  label: string;
  node_type: string;
  parent_id?: string;
  timestamp?: number;
  total_weight?: number;
}

export interface GraphEdge {
  source: string;
  target: string;
  relation: string;
  weight: number;
}

export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface CoreValueContext {
  episode_parent_id: string;
  context: string;
  weight: number;
  timestamp: number;
}

export interface CoreValueDetail {
  value_name: string;
  total_weight: number;
  contexts: CoreValueContext[];
}

export interface ConversationMessage {
  role: string;
  content: string;
  timestamp: number;
}

export interface EpisodeDetail {
  parent_id: string;
  timestamp: number;
  core_values: string[];
  persons: string[];
  messages: ConversationMessage[];
}

export interface Episode {
  id: string;
  timestamp: number;
  text: string;
  emotion_type?: string;
  score?: number;
}

export const chatApi = {
  sendMessage: async (message: ChatMessage): Promise<ChatResponse> => {
    const response = await api.post<ChatResponse>('/chat/message', message);
    return response.data;
  },
};

export const insightsApi = {
  getGraph: async (): Promise<GraphData> => {
    const response = await api.get<GraphData>('/insights/graph');
    return response.data;
  },
  getCoreValueGraph: async (): Promise<GraphData> => {
    const response = await api.get<GraphData>('/insights/core-value-graph');
    return response.data;
  },
  getCoreValueDetail: async (valueName: string): Promise<CoreValueDetail> => {
    const response = await api.get<CoreValueDetail>(`/insights/core-values/${encodeURIComponent(valueName)}`);
    return response.data;
  },
};

export const episodesApi = {
  getEpisodes: async (): Promise<Episode[]> => {
    const response = await api.get<Episode[]>('/episodes');
    return response.data;
  },
  getEpisodeById: async (id: string): Promise<Episode> => {
    const response = await api.get<Episode>(`/episodes/${id}`);
    return response.data;
  },
  getEpisodeByParentId: async (parentId: string): Promise<EpisodeDetail> => {
    const response = await api.get<EpisodeDetail>(`/episodes/parent/${parentId}`);
    return response.data;
  },
};
