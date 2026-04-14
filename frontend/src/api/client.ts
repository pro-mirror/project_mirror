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
};
