export interface Meeting {
  id: string;
  title: string;
  created_at: string;
  audio_path?: string;
  duration_secs?: number;
  transcript?: string;
  summary?: MeetingSummary;
}

export interface MeetingSummary {
  summary: string;
  action_items: string[];
  key_decisions: string[];
  participants: string[];
}

export interface Note {
  id: string;
  meeting_id: string;
  content: string;
  timestamps: number[];
  updated_at: string;
}

export interface Template {
  id: string;
  name: string;
  structure: {
    sections: string[];
  };
}

export interface TranscriptSegment {
  start_time: number;
  end_time: number;
  text: string;
  speaker?: string;
}

export interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
}
