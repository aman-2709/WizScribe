<script lang="ts">
  import { onMount } from 'svelte';
  import { Play, Pause, Volume2 } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';

  export let audioPath: string;
  
  let audio: HTMLAudioElement;
  let isPlaying = false;
  let currentTime = 0;
  let duration = 0;
  let volume = 1;
  let progress = 0;

  onMount(async () => {
    try {
      duration = await invoke('get_audio_duration', { audioPath });
    } catch (e) {
      console.error('Failed to get audio duration:', e);
    }
  });

  function togglePlay() {
    if (!audio) return;
    
    if (isPlaying) {
      audio.pause();
    } else {
      audio.play();
    }
  }

  function onTimeUpdate() {
    if (audio) {
      currentTime = audio.currentTime;
      progress = duration > 0 ? (currentTime / duration) * 100 : 0;
    }
  }

  function onLoadedMetadata() {
    if (audio) {
      duration = audio.duration;
    }
  }

  function onEnded() {
    isPlaying = false;
    currentTime = 0;
    progress = 0;
  }

  function seek(e: MouseEvent) {
    if (!audio) return;
    const rect = (e.currentTarget as HTMLDivElement).getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const percentage = clickX / rect.width;
    audio.currentTime = percentage * duration;
  }

  function formatTime(seconds: number): string {
    if (!isFinite(seconds)) return '0:00';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  // Convert file path to audio URL
  $: audioUrl = audioPath ? `file://${audioPath}` : '';
</script>

{#if audioPath}
  <div class="bg-surface-100 rounded-lg p-4">
    <audio
      bind:this={audio}
      src={audioUrl}
      on:play={() => isPlaying = true}
      on:pause={() => isPlaying = false}
      on:timeupdate={onTimeUpdate}
      on:loadedmetadata={onLoadedMetadata}
      on:ended={onEnded}
    ></audio>

    <div class="flex items-center gap-4">
      <button
        class="btn-ghost p-2 rounded-full"
        on:click={togglePlay}
      >
        {#if isPlaying}
          <Pause class="w-5 h-5" />
        {:else}
          <Play class="w-5 h-5" />
        {/if}
      </button>

      <div class="flex-1">
        <div
          class="h-2 bg-surface-300 rounded-full cursor-pointer overflow-hidden"
          on:click={seek}
        >
          <div
            class="h-full bg-primary-500 rounded-full transition-all"
            style="width: {progress}%"
          ></div>
        </div>
      </div>

      <div class="text-sm text-surface-600 min-w-[80px] text-center">
        {formatTime(currentTime)} / {formatTime(duration)}
      </div>

      <div class="flex items-center gap-2">
        <Volume2 class="w-4 h-4 text-surface-500" />
        <input
          type="range"
          min="0"
          max="1"
          step="0.1"
          bind:value={volume}
          on:input={() => { if (audio) audio.volume = volume; }}
          class="w-20 accent-primary-500"
        />
      </div>
    </div>
  </div>
{:else}
  <div class="bg-surface-100 rounded-lg p-4 text-center text-surface-500">
    No audio recording available
  </div>
{/if}
