<script lang="ts">
  import { Mic, MicOff, Pause, Play, Square, Loader2 } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { recordingState, currentMeeting } from '$lib/stores';
  import type { Meeting } from '$lib/types';

  let isLoading = false;
  let error: string | null = null;

  async function startRecording() {
    if (isLoading) return;
    isLoading = true;
    error = null;

    try {
      // Create a new meeting first
      const meeting: Meeting = await invoke('create_meeting', {
        title: `Meeting ${new Date().toLocaleString()}`,
      });
      
      currentMeeting.set(meeting);
      
      // Start recording
      const audioPath: string = await invoke('start_recording', {
        meetingId: meeting.id,
      });
      
      recordingState.setState({
        state: 'recording',
        meeting_id: meeting.id,
      });
    } catch (e) {
      error = String(e);
      console.error('Failed to start recording:', e);
    } finally {
      isLoading = false;
    }
  }

  async function stopRecording() {
    if (isLoading) return;
    isLoading = true;
    error = null;

    try {
      const [meetingId, duration]: [string, number] = await invoke('stop_recording');
      
      recordingState.setState({
        state: 'idle',
        meeting_id: null,
      });
      
      // Update the meeting with duration
      if ($currentMeeting) {
        currentMeeting.update(m => m ? { ...m, duration } : null);
      }
    } catch (e) {
      error = String(e);
      console.error('Failed to stop recording:', e);
    } finally {
      isLoading = false;
    }
  }

  async function pauseRecording() {
    if (isLoading) return;
    isLoading = true;

    try {
      await invoke('pause_recording');
      recordingState.update(state => ({ ...state, state: 'paused' }));
    } catch (e) {
      error = String(e);
      console.error('Failed to pause recording:', e);
    } finally {
      isLoading = false;
    }
  }

  async function resumeRecording() {
    if (isLoading) return;
    isLoading = true;

    try {
      await invoke('resume_recording');
      recordingState.update(state => ({ ...state, state: 'recording' }));
    } catch (e) {
      error = String(e);
      console.error('Failed to resume recording:', e);
    } finally {
      isLoading = false;
    }
  }
</script>

<div class="flex flex-col items-center gap-4">
  {#if error}
    <div class="text-red-500 text-sm">{error}</div>
  {/if}

  <div class="flex items-center gap-3">
    {#if $recordingState.state === 'idle'}
      <button
        class="btn-primary w-16 h-16 rounded-full shadow-lg hover:shadow-xl transform hover:scale-105"
        on:click={startRecording}
        disabled={isLoading}
        title="Start Recording"
      >
        {#if isLoading}
          <Loader2 class="w-8 h-8 animate-spin" />
        {:else}
          <Mic class="w-8 h-8" />
        {/if}
      </button>
    {:else if $recordingState.state === 'recording'}
      <button
        class="btn-secondary w-12 h-12 rounded-full"
        on:click={pauseRecording}
        disabled={isLoading}
        title="Pause"
      >
        <Pause class="w-6 h-6" />
      </button>
      
      <button
        class="btn-danger w-16 h-16 rounded-full shadow-lg animate-pulse"
        on:click={stopRecording}
        disabled={isLoading}
        title="Stop Recording"
      >
        {#if isLoading}
          <Loader2 class="w-8 h-8 animate-spin" />
        {:else}
          <Square class="w-8 h-8" />
        {/if}
      </button>
    {:else if $recordingState.state === 'paused'}
      <button
        class="btn-primary w-12 h-12 rounded-full"
        on:click={resumeRecording}
        disabled={isLoading}
        title="Resume"
      >
        <Play class="w-6 h-6" />
      </button>
      
      <button
        class="btn-danger w-16 h-16 rounded-full shadow-lg"
        on:click={stopRecording}
        disabled={isLoading}
        title="Stop Recording"
      >
        {#if isLoading}
          <Loader2 class="w-8 h-8 animate-spin" />
        {:else}
          <Square class="w-8 h-8" />
        {/if}
      </button>
    {/if}
  </div>

  {#if $recordingState.state === 'recording'}
    <div class="flex items-center gap-2 text-red-500">
      <div class="w-3 h-3 bg-red-500 rounded-full animate-pulse"></div>
      <span class="text-sm font-medium">Recording...</span>
    </div>
  {:else if $recordingState.state === 'paused'}
    <div class="flex items-center gap-2 text-amber-500">
      <Pause class="w-4 h-4" />
      <span class="text-sm font-medium">Paused</span>
    </div>
  {/if}
</div>
