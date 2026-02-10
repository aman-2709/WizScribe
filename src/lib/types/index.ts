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

export interface AudioDevice {
  index: number;
  name: string;
  is_monitor: boolean;
}

// Dual Audio Types (001-dual-audio-speaker-id)

export interface SpeakerSegment {
  speaker: 'Me' | 'Them';
  text: string;
  start_ms: number;
  end_ms: number;
  is_overlapping: boolean;
}

export interface SpeakerTranscript {
  version: number;
  mic_device: string;
  system_device: string;
  has_dual_audio: boolean;
  segments: SpeakerSegment[];
}

export interface DualAudioConfig {
  mic_device_index: number | null;
  system_device_index: number | null;
}

export interface DualRecordingStatus {
  audio_path: string;
  mic_active: boolean;
  system_active: boolean;
  mic_device: string;
  system_device: string;
}

export interface DualRecordingResult {
  meeting_id: string;
  duration_secs: number;
  is_dual_audio: boolean;
  mic_captured: boolean;
  system_captured: boolean;
}

export interface AudioSourceError {
  source: 'mic' | 'system';
  error: string;
  timestamp: number;
  recording_continues: boolean;
}

// Type guard for SpeakerTranscript vs plain string
export function isSpeakerTranscript(transcript: unknown): transcript is SpeakerTranscript {
  return (
    typeof transcript === 'object' &&
    transcript !== null &&
    'segments' in transcript &&
    Array.isArray((transcript as SpeakerTranscript).segments)
  );
}
