import { writable } from 'svelte/store';
import type { Meeting, Note, Template, AudioDevice } from '$lib/types';

export const meetings = writable<Meeting[]>([]);
export const currentMeeting = writable<Meeting | null>(null);
export const currentNote = writable<Note | null>(null);
export const templates = writable<Template[]>([]);
export const isRecording = writable<boolean>(false);
export const recordingTime = writable<number>(0);
export const isTranscribing = writable<boolean>(false);
export const isGeneratingSummary = writable<boolean>(false);

// Recording state store with custom methods
interface RecordingStateData {
  state: 'idle' | 'recording' | 'paused';
  meeting_id: string | null;
  is_dual_mode: boolean;
  mic_active: boolean;
  system_active: boolean;
  mic_device: string | null;
  system_device: string | null;
}

function createRecordingState() {
  const { subscribe, set, update } = writable<RecordingStateData>({
    state: 'idle',
    meeting_id: null,
    is_dual_mode: false,
    mic_active: false,
    system_active: false,
    mic_device: null,
    system_device: null,
  });

  return {
    subscribe,
    update,
    setState: (data: Partial<RecordingStateData>) => {
      update(state => ({ ...state, ...data }));
    },
    reset: () => {
      set({
        state: 'idle',
        meeting_id: null,
        is_dual_mode: false,
        mic_active: false,
        system_active: false,
        mic_device: null,
        system_device: null,
      });
    },
  };
}

export const recordingState = createRecordingState();

// Audio devices stores for dual audio configuration
export const microphoneDevices = writable<AudioDevice[]>([]);
export const monitorDevices = writable<AudioDevice[]>([]);
