<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { Calendar, Clock, FileAudio, Trash2, MoreVertical, Edit2, FileText, Download } from 'lucide-svelte';
  import type { Meeting } from '$lib/types';
  import { meetings } from '$lib/stores';

  export let selectedMeetingId: string | null = null;

  let isLoading = false;
  let editingId: string | null = null;
  let editingTitle = '';

  onMount(async () => {
    await loadMeetings();
  });

  async function loadMeetings() {
    isLoading = true;
    try {
      const data: Meeting[] = await invoke('get_meetings');
      meetings.set(data);
    } catch (e) {
      console.error('Failed to load meetings:', e);
    } finally {
      isLoading = false;
    }
  }

  async function deleteMeeting(id: string) {
    if (!confirm('Are you sure you want to delete this meeting?')) return;
    
    try {
      await invoke('delete_meeting', { id });
      meetings.removeMeeting(id);
    } catch (e) {
      console.error('Failed to delete meeting:', e);
    }
  }

  function startEditing(meeting: Meeting) {
    editingId = meeting.id;
    editingTitle = meeting.title;
  }

  async function saveTitle(id: string) {
    if (!editingTitle.trim()) {
      editingId = null;
      return;
    }

    try {
      await invoke('update_meeting_title', { 
        id, 
        title: editingTitle.trim() 
      });
      meetings.updateMeeting(id, { title: editingTitle.trim() });
    } catch (e) {
      console.error('Failed to update title:', e);
    } finally {
      editingId = null;
    }
  }

  function formatDuration(seconds: number | null): string {
    if (!seconds) return '';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));

    if (days === 0) {
      return 'Today';
    } else if (days === 1) {
      return 'Yesterday';
    } else if (days < 7) {
      return date.toLocaleDateString('en-US', { weekday: 'long' });
    } else {
      return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
    }
  }

  async function exportMeeting(meeting: Meeting) {
    try {
      const markdown: string = await invoke('export_to_markdown', {
        meetingId: meeting.id,
      });
      
      // Create and download the file
      const blob = new Blob([markdown], { type: 'text/markdown' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${meeting.title.replace(/[^a-z0-9]/gi, '_').toLowerCase()}.md`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (e) {
      console.error('Failed to export meeting:', e);
    }
  }
</script>

<div class="h-full overflow-y-auto scrollbar-thin">
  {#if isLoading}
    <div class="flex items-center justify-center h-32">
      <div class="animate-spin w-6 h-6 border-2 border-primary-500 border-t-transparent rounded-full"></div>
    </div>
  {:else if $meetings.length === 0}
    <div class="text-center p-8 text-surface-500">
      <FileText class="w-12 h-12 mx-auto mb-3 opacity-50" />
      <p>No meetings yet</p>
      <p class="text-sm mt-1">Create a new meeting to get started</p>
    </div>
  {:else}
    <div class="divide-y divide-surface-200">
      {#each $meetings as meeting}
        <div 
          class="p-4 hover:bg-surface-100 cursor-pointer transition-colors {selectedMeetingId === meeting.id ? 'bg-primary-50 border-r-2 border-primary-500' : ''}"
          on:click={() => selectedMeetingId = meeting.id}
        >
          <div class="flex items-start justify-between gap-2">
            <div class="flex-1 min-w-0">
              {#if editingId === meeting.id}
                <input
                  type="text"
                  bind:value={editingTitle}
                  on:blur={() => saveTitle(meeting.id)}
                  on:keydown={(e) => e.key === 'Enter' && saveTitle(meeting.id)}
                  class="input text-sm py-1"
                  autofocus
                />
              {:else}
                <h4 class="font-medium text-surface-800 truncate" title={meeting.title}>
                  {meeting.title}
                </h4>
              {/if}
              
              <div class="flex items-center gap-3 mt-1 text-xs text-surface-500">
                <span class="flex items-center gap-1">
                  <Calendar class="w-3 h-3" />
                  {formatDate(meeting.created_at)}
                </span>
                
                {#if meeting.duration}
                  <span class="flex items-center gap-1">
                    <Clock class="w-3 h-3" />
                    {formatDuration(meeting.duration)}
                  </span>
                {/if}
              </div>

              <div class="flex items-center gap-2 mt-2">
                {#if meeting.audio_path}
                  <span class="text-xs bg-surface-200 text-surface-600 px-2 py-0.5 rounded flex items-center gap-1">
                    <FileAudio class="w-3 h-3" />
                    Audio
                  </span>
                {/if}
                {#if meeting.transcript}
                  <span class="text-xs bg-primary-100 text-primary-700 px-2 py-0.5 rounded flex items-center gap-1">
                    <FileText class="w-3 h-3" />
                    Transcript
                  </span>
                {/if}
              </div>
            </div>

            <div class="flex items-center gap-1">
              <button
                class="p-1.5 text-surface-400 hover:text-surface-600 hover:bg-surface-200 rounded"
                on:click|stopPropagation={() => startEditing(meeting)}
                title="Rename"
              >
                <Edit2 class="w-3.5 h-3.5" />
              </button>
              <button
                class="p-1.5 text-surface-400 hover:text-primary-600 hover:bg-primary-100 rounded"
                on:click|stopPropagation={() => exportMeeting(meeting)}
                title="Export"
              >
                <Download class="w-3.5 h-3.5" />
              </button>
              <button
                class="p-1.5 text-surface-400 hover:text-red-600 hover:bg-red-100 rounded"
                on:click|stopPropagation={() => deleteMeeting(meeting.id)}
                title="Delete"
              >
                <Trash2 class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
