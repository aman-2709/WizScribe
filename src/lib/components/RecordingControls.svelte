<script lang="ts">
  import { Mic, MicOff, Pause, Play, Square, Loader2, Users } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';
  import { recordingState, currentMeeting } from '$lib/stores';
  import { startDualRecording, stopDualRecording } from '$lib/api';
  import type { Meeting, DualRecordingStatus, AudioSourceError } from '$lib/types';

  let isLoading = false;
  let error: string | null = null;
  let audioSourceWarning: string | null = null;
  let unlisten: (() => void) | null = null;

  onMount(async () => {
    // Listen for audio source errors
    unlisten = await listen<AudioSourceError>('audio-source-error', (event) => {
      const { source, error: errMsg, recording_continues } = event.payload;
      if (recording_continues) {
        audioSourceWarning = `${source === 'mic' ? 'Microphone' : 'System audio'} error: ${errMsg}. Recording continues with available source.`;
      } else {
        error = `${source === 'mic' ? 'Microphone' : 'System audio'} failed: ${errMsg}`;
      }
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  async function startRecording() {
    if (isLoading) return;
    isLoading = true;
    error = null;
    audioSourceWarning = null;

    try {
      // Create a new meeting first
      const meeting: Meeting = await invoke('create_meeting', {
        title: `Meeting ${new Date().toLocaleString()}`,
      });

      currentMeeting.set(meeting);

      // Start dual recording
      const status: DualRecordingStatus = await startDualRecording(meeting.id);

      recordingState.setState({
        state: 'recording',
        meeting_id: meeting.id,
        is_dual_mode: true,
        mic_active: status.mic_active,
        system_active: status.system_active,
        mic_device: status.mic_device,
        system_device: status.system_device,
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
      const result = await stopDualRecording();

      recordingState.reset();

      // Update the meeting with duration
      if ($currentMeeting) {
        currentMeeting.update(m => m ? { ...m, duration_secs: result.duration_secs } : null);
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

  {#if audioSourceWarning}
    <div class="text-amber-500 text-sm bg-amber-500/10 px-3 py-2 rounded-lg">{audioSourceWarning}</div>
  {/if}

  <div class="flex items-center gap-3">
    {#if $recordingState.state === 'idle'}
      <button
        class="btn-primary w-16 h-16 rounded-full shadow-lg hover:shadow-xl transform hover:scale-105"
        on:click={startRecording}
        disabled={isLoading}
        title="Start Dual Recording (Mic + System Audio)"
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
    <div class="flex flex-col items-center gap-2">
      <div class="flex items-center gap-2 text-red-500">
        <div class="w-3 h-3 bg-red-500 rounded-full animate-pulse"></div>
        <span class="text-sm font-medium">Recording...</span>
      </div>
      {#if $recordingState.is_dual_mode}
        <div class="flex items-center gap-4 text-xs text-gray-400">
          <div class="flex items-center gap-1" class:text-green-400={$recordingState.mic_active} class:text-red-400={!$recordingState.mic_active}>
            <Mic class="w-3 h-3" />
            <span>Mic</span>
          </div>
          <div class="flex items-center gap-1" class:text-green-400={$recordingState.system_active} class:text-red-400={!$recordingState.system_active}>
            <Users class="w-3 h-3" />
            <span>System</span>
          </div>
        </div>
      {/if}
    </div>
  {:else if $recordingState.state === 'paused'}
    <div class="flex items-center gap-2 text-amber-500">
      <Pause class="w-4 h-4" />
      <span class="text-sm font-medium">Paused</span>
    </div>
  {/if}
</div>
