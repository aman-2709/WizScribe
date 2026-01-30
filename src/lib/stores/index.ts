import { writable } from 'svelte/store';
import type { Meeting, Note, Template } from '$lib/types';

export const meetings = writable<Meeting[]>([]);
export const currentMeeting = writable<Meeting | null>(null);
export const currentNote = writable<Note | null>(null);
export const templates = writable<Template[]>([]);
export const isRecording = writable<boolean>(false);
export const recordingTime = writable<number>(0);
export const isTranscribing = writable<boolean>(false);
export const isGeneratingSummary = writable<boolean>(false);
