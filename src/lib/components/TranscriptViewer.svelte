<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { Loader2, RefreshCw, FileText, Clock } from 'lucide-svelte';
  import type { Meeting } from '$lib/types';
  import { currentMeeting } from '$lib/stores';

  export let meeting: Meeting;

  let isTranscribing = false;
  let isGeneratingSummary = false;
  let error: string | null = null;

  async function transcribe() {
    if (!meeting.audio_path || isTranscribing) return;
    
    isTranscribing = true;
    error = null;

    try {
      const transcript: string = await invoke('transcribe_audio', {
        audioPath: meeting.audio_path,
      });
      
      await invoke('update_transcript', {
        meetingId: meeting.id,
        transcript,
      });
      
      currentMeeting.update(m => m ? { ...m, transcript } : null);
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
      const summary: string = await invoke('generate_summary', {
        transcript: meeting.transcript,
      });
      
      await invoke('update_summary', {
        meetingId: meeting.id,
        summary,
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

  $: parsedTranscript = meeting.transcript ? parseTranscript(meeting.transcript) : [];
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
      <h3 class="text-lg font-medium text-surface-800">Transcript</h3>
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
        <div class="text-sm text-primary-700 whitespace-pre-wrap">{meeting.summary}</div>
      </div>
    {/if}

    <div class="flex-1 overflow-y-auto scrollbar-thin space-y-2">
      {#each parsedTranscript as segment}
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
</div>
