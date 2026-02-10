import { invoke } from '@tauri-apps/api/core';
import type {
  Meeting,
  Note,
  Template,
  AudioDevice,
  DualAudioConfig,
  DualRecordingStatus,
  DualRecordingResult,
  SpeakerTranscript
} from '$lib/types';

// Meeting API
export async function createMeeting(title: string): Promise<Meeting> {
  return invoke('create_meeting', { title });
}

export async function getMeeting(id: string): Promise<Meeting | null> {
  return invoke('get_meeting', { id });
}

export async function listMeetings(): Promise<Meeting[]> {
  return invoke('list_meetings');
}

// Alias for backward compatibility
export const getMeetings = listMeetings;

export async function updateMeetingTitle(id: string, title: string): Promise<void> {
  return invoke('update_meeting_title', { id, title });
}

export async function deleteMeeting(id: string): Promise<void> {
  return invoke('delete_meeting', { id });
}

// Recording API
export async function startRecording(meetingId: string): Promise<string> {
  return invoke('start_recording', { meetingId });
}

export async function stopRecording(): Promise<[string, number]> {
  return invoke('stop_recording');
}

export async function pauseRecording(): Promise<void> {
  return invoke('pause_recording');
}

export async function resumeRecording(): Promise<void> {
  return invoke('resume_recording');
}

export async function getRecordingState(): Promise<{ state: string; meeting_id: string | null }> {
  return invoke('get_recording_state');
}

export async function isRecording(): Promise<boolean> {
  const state = await getRecordingState();
  return state.state === 'recording';
}

// Audio Device API
export async function listAudioDevices(): Promise<AudioDevice[]> {
  return invoke('list_audio_devices');
}

export async function setRecordingDevice(deviceIndex: number | null): Promise<void> {
  return invoke('set_recording_device', { deviceIndex });
}

export async function getSelectedAudioDevice(): Promise<number | null> {
  return invoke('get_selected_audio_device');
}

// Dual Audio API
export async function startDualRecording(meetingId: string): Promise<DualRecordingStatus> {
  return invoke('start_dual_recording', { meetingId });
}

export async function stopDualRecording(): Promise<DualRecordingResult> {
  return invoke('stop_dual_recording');
}

export async function getDualAudioConfig(): Promise<DualAudioConfig> {
  return invoke('get_dual_audio_config');
}

export async function setDualAudioConfig(
  micDeviceIndex: number | null,
  systemDeviceIndex: number | null
): Promise<void> {
  return invoke('set_dual_audio_config', { micDeviceIndex, systemDeviceIndex });
}

export async function getAudioDevicesByType(deviceType: 'microphone' | 'monitor'): Promise<AudioDevice[]> {
  return invoke('get_audio_devices_by_type', { deviceType });
}

export async function transcribeDualAudio(meetingId: string, audioPath: string): Promise<SpeakerTranscript> {
  return invoke('transcribe_dual_audio', { meetingId, audioPath });
}

export async function getDualRecordingState(): Promise<{
  is_recording: boolean;
  mic_active: boolean;
  system_active: boolean;
  mic_device: string | null;
  system_device: string | null;
  meeting_id: string | null;
}> {
  return invoke('get_dual_recording_state');
}

// Transcript API
export async function updateTranscript(meetingId: string, transcript: string): Promise<void> {
  return invoke('update_transcript', { meetingId, transcript });
}

export async function updateSummary(meetingId: string, summary: string): Promise<void> {
  return invoke('update_summary', { meetingId, summary });
}

// Note API
export async function getNote(meetingId: string): Promise<Note | null> {
  return invoke('get_note', { meetingId });
}

export async function updateNote(meetingId: string, content: string, timestamps: number[] = []): Promise<void> {
  return invoke('update_note', { meetingId, content, timestamps });
}

// Alias for backward compatibility
export async function saveNote(meetingId: string, content: string, timestampsJson: string): Promise<Note> {
  const timestamps = JSON.parse(timestampsJson || '[]');
  await updateNote(meetingId, content, timestamps);
  return getNote(meetingId) as Promise<Note>;
}

// Template API
export async function listTemplates(): Promise<Template[]> {
  return invoke('list_templates');
}

// Alias for backward compatibility
export const getTemplates = listTemplates;

export async function getTemplate(id: string): Promise<Template | null> {
  return invoke('get_template', { id });
}

export async function createTemplate(name: string, structureJson: string): Promise<Template> {
  return invoke('create_template', { name, structureJson });
}

export async function deleteTemplate(id: string): Promise<void> {
  return invoke('delete_template', { id });
}

// Transcription API
export async function transcribeAudio(meetingId: string, audioPath: string): Promise<string> {
  return invoke('transcribe_audio', { meetingId, audioPath });
}

export async function transcribeMeeting(meetingId: string): Promise<string> {
  // Get meeting to find audio path
  const meeting = await getMeeting(meetingId);
  if (!meeting?.audio_path) {
    throw new Error('Meeting has no audio recording');
  }
  return transcribeAudio(meetingId, meeting.audio_path);
}

export async function getAudioDuration(audioPath: string): Promise<number> {
  return invoke('get_audio_duration', { audioPath });
}

// Whisper model API
export async function isWhisperModelAvailable(): Promise<boolean> {
  return invoke('is_whisper_model_available');
}

export async function getWhisperModelPath(): Promise<string> {
  return invoke('get_whisper_model_path');
}

// AI API
export async function generateSummary(meetingId: string, transcript: string): Promise<string> {
  return invoke('generate_summary', { meetingId, transcript });
}

export async function chatWithAI(transcript: string, question: string): Promise<string> {
  return invoke('chat_with_ai', { transcript, question });
}

export async function askQuestion(transcript: string, question: string): Promise<string> {
  return chatWithAI(transcript, question);
}

export async function extractActionItems(transcript: string): Promise<string[]> {
  return invoke('extract_action_items', { transcript });
}

export async function draftFollowupEmail(transcript: string, summary: string): Promise<string> {
  const prompt = `Based on this meeting transcript and summary, draft a professional follow-up email to send to the attendees.

Summary:
${summary}

Transcript:
${transcript}

Please write a concise follow-up email that:
1. Thanks attendees for their time
2. Summarizes key discussion points
3. Lists action items with owners (if mentioned)
4. Notes any deadlines or next steps`;

  return chatWithAI(transcript, prompt);
}

export async function setAiApiKey(apiKey: string, provider: string): Promise<void> {
  return invoke('set_ai_api_key', { apiKey, provider });
}

// Export API
export async function exportToMarkdown(meetingId: string): Promise<string> {
  const meeting = await getMeeting(meetingId);
  if (!meeting) {
    throw new Error('Meeting not found');
  }

  const note = await getNote(meetingId);

  let markdown = `# ${meeting.title}\n\n`;
  markdown += `**Date:** ${new Date(meeting.created_at).toLocaleString()}\n\n`;

  if (meeting.duration_secs) {
    const mins = Math.floor(meeting.duration_secs / 60);
    const secs = meeting.duration_secs % 60;
    markdown += `**Duration:** ${mins}:${secs.toString().padStart(2, '0')}\n\n`;
  }

  if (meeting.summary) {
    markdown += `## Summary\n\n${meeting.summary}\n\n`;
  }

  if (note?.content) {
    markdown += `## Notes\n\n${note.content}\n\n`;
  }

  if (meeting.transcript) {
    markdown += `## Transcript\n\n${meeting.transcript}\n`;
  }

  return markdown;
}
