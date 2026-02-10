<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { Loader2, RefreshCw, FileText, Clock, Mic, Users, MessageCircle } from 'lucide-svelte';
  import type { Meeting, SpeakerTranscript, SpeakerSegment } from '$lib/types';
  import { isSpeakerTranscript } from '$lib/types';
  import { currentMeeting } from '$lib/stores';
  import { transcribeDualAudio } from '$lib/api';

  export let meeting: Meeting;

  let isTranscribing = false;
  let isGeneratingSummary = false;
  let error: string | null = null;

  // Parsed transcript state
  let speakerTranscript: SpeakerTranscript | null = null;
  let plainTextSegments: Array<{ time: string; text: string }> = [];
  let isDualAudio = false;

  // Parse transcript when meeting changes
  $: {
    if (meeting.transcript) {
      try {
        const parsed = JSON.parse(meeting.transcript);
        if (isSpeakerTranscript(parsed)) {
          speakerTranscript = parsed;
          isDualAudio = true;
          plainTextSegments = [];
        } else {
          throw new Error('Not a speaker transcript');
        }
      } catch {
        // Plain text transcript
        speakerTranscript = null;
        isDualAudio = false;
        plainTextSegments = parseTranscript(meeting.transcript);
      }
    } else {
      speakerTranscript = null;
      isDualAudio = false;
      plainTextSegments = [];
    }
  }

  async function transcribe() {
    if (!meeting.audio_path || isTranscribing) return;

    isTranscribing = true;
    error = null;

    try {
      // Try dual audio transcription first if it's a stereo file
      try {
        const transcript = await transcribeDualAudio(meeting.id, meeting.audio_path);
        const transcriptJson = JSON.stringify(transcript);
        currentMeeting.update(m => m ? { ...m, transcript: transcriptJson } : null);
      } catch (dualError) {
        // Fall back to regular transcription
        console.log('Dual transcription failed, falling back to mono:', dualError);
        const transcript: string = await invoke('transcribe_audio', {
          meetingId: meeting.id,
          audioPath: meeting.audio_path,
        });
        currentMeeting.update(m => m ? { ...m, transcript } : null);
      }
    } catch (e) {
      error = String(e);
      console.error('Failed to transcribe:', e);
    } finally {
      isTranscribing = false;
    }
  }

  async function generateSummary() {
    if (!meeting.transcript || isGeneratingSummary) return;

    isGeneratingSummary = true;
    error = null;

    try {
      // For speaker transcripts, flatten to text for summary
      let transcriptText = meeting.transcript;
      if (speakerTranscript) {
        transcriptText = speakerTranscript.segments
          .map(s => `[${s.speaker}]: ${s.text}`)
          .join('\n');
      }

      const summary: string = await invoke('generate_summary', {
        meetingId: meeting.id,
        transcript: transcriptText,
      });

      currentMeeting.update(m => m ? { ...m, summary } : null);
    } catch (e) {
      error = String(e);
      console.error('Failed to generate summary:', e);
    } finally {
      isGeneratingSummary = false;
    }
  }

  function parseTranscript(transcript: string): Array<{ time: string; text: string }> {
    const lines = transcript.split('\n').filter(line => line.trim());
    return lines.map(line => {
      const match = line.match(/^\[(\d{2}:\d{2}\.\d{3})\].*?\]\s*(.+)$/);
      if (match) {
        return { time: match[1], text: match[2].trim() };
      }
      return { time: '', text: line };
    });
  }

  function formatTime(ms: number): string {
    const totalSeconds = Math.floor(ms / 1000);
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    const milliseconds = ms % 1000;
    return `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}.${milliseconds.toString().padStart(3, '0')}`;
  }
</script>

<div class="flex flex-col h-full">
  {#if error}
    <div class="bg-red-50 text-red-600 p-3 rounded-lg mb-4 text-sm">
      {error}
    </div>
  {/if}

  {#if !meeting.transcript}
    <div class="flex flex-col items-center justify-center h-full p-8 text-center">
      <FileText class="w-16 h-16 text-surface-300 mb-4" />
      <h3 class="text-lg font-medium text-surface-700 mb-2">No Transcript Yet</h3>
      <p class="text-surface-500 mb-4 max-w-md">
        {#if meeting.audio_path}
          Transcribe this meeting to get a text version of the conversation.
        {:else}
          Record audio for this meeting first, then transcribe it.
        {/if}
      </p>
      {#if meeting.audio_path}
        <button
          class="btn-primary"
          on:click={transcribe}
          disabled={isTranscribing}
        >
          {#if isTranscribing}
            <Loader2 class="w-4 h-4 animate-spin" />
            Transcribing...
          {:else}
            <RefreshCw class="w-4 h-4" />
            Transcribe Audio
          {/if}
        </button>
      {/if}
    </div>
  {:else}
    <div class="flex items-center justify-between mb-4">
      <div class="flex items-center gap-2">
        <h3 class="text-lg font-medium text-surface-800">Transcript</h3>
        {#if isDualAudio}
          <span class="text-xs bg-blue-100 text-blue-700 px-2 py-0.5 rounded-full">Speaker Labels</span>
        {/if}
      </div>
      <div class="flex gap-2">
        {#if meeting.audio_path}
          <button
            class="btn-secondary text-sm"
            on:click={transcribe}
            disabled={isTranscribing}
          >
            {#if isTranscribing}
              <Loader2 class="w-4 h-4 animate-spin" />
            {:else}
              <RefreshCw class="w-4 h-4" />
            {/if}
            Retranscribe
          </button>
        {/if}
        {#if !meeting.summary}
          <button
            class="btn-primary text-sm"
            on:click={generateSummary}
            disabled={isGeneratingSummary}
          >
            {#if isGeneratingSummary}
              <Loader2 class="w-4 h-4 animate-spin" />
            {/if}
            Generate Summary
          </button>
        {/if}
      </div>
    </div>

    {#if meeting.summary}
      <div class="bg-primary-50 border border-primary-200 rounded-lg p-4 mb-4">
        <h4 class="text-sm font-medium text-primary-800 mb-2">AI Summary</h4>
        <div class="text-sm text-primary-700 whitespace-pre-wrap">{typeof meeting.summary === 'string' ? meeting.summary : JSON.stringify(meeting.summary)}</div>
      </div>
    {/if}

    <!-- Speaker-labeled transcript (dual audio) -->
    {#if isDualAudio && speakerTranscript}
      <div class="mb-4 text-xs text-gray-500 flex items-center gap-4">
        <span class="flex items-center gap-1">
          <Mic class="w-3 h-3" />
          {speakerTranscript.mic_device}
        </span>
        <span class="flex items-center gap-1">
          <Users class="w-3 h-3" />
          {speakerTranscript.system_device}
        </span>
      </div>
      <div class="flex-1 overflow-y-auto scrollbar-thin space-y-3">
        {#each speakerTranscript.segments as segment}
          <div
            class="flex gap-3 p-3 rounded-lg transition-colors"
            class:bg-blue-50={segment.speaker === 'Me'}
            class:ml-8={segment.speaker === 'Me'}
            class:bg-gray-100={segment.speaker === 'Them'}
            class:mr-8={segment.speaker === 'Them'}
          >
            <div class="flex-shrink-0">
              {#if segment.speaker === 'Me'}
                <div class="w-8 h-8 rounded-full bg-blue-500 flex items-center justify-center">
                  <Mic class="w-4 h-4 text-white" />
                </div>
              {:else}
                <div class="w-8 h-8 rounded-full bg-gray-500 flex items-center justify-center">
                  <Users class="w-4 h-4 text-white" />
                </div>
              {/if}
            </div>
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 mb-1">
                <span class="font-medium text-sm" class:text-blue-700={segment.speaker === 'Me'} class:text-gray-700={segment.speaker === 'Them'}>
                  {segment.speaker}
                </span>
                <span class="text-xs text-gray-400 font-mono">
                  {formatTime(segment.start_ms)}
                </span>
                {#if segment.is_overlapping}
                  <span class="text-xs bg-amber-100 text-amber-700 px-1.5 py-0.5 rounded flex items-center gap-1">
                    <MessageCircle class="w-3 h-3" />
                    overlapping
                  </span>
                {/if}
              </div>
              <p class="text-sm text-gray-700 break-words">{segment.text}</p>
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <!-- Plain text transcript (legacy format) -->
      <div class="flex-1 overflow-y-auto scrollbar-thin space-y-2">
        {#each plainTextSegments as segment}
          <div class="flex gap-3 p-2 rounded-lg hover:bg-surface-100 transition-colors">
            <div class="flex-shrink-0 text-xs text-surface-400 font-mono pt-0.5">
              <Clock class="w-3 h-3 inline mr-1" />
              {segment.time}
            </div>
            <div class="text-sm text-surface-700">{segment.text}</div>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>
