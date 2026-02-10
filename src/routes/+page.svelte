<script lang="ts">
  import { onMount } from 'svelte';
  import { meetings, templates, isRecording, recordingTime } from '$lib/stores';
  import { listMeetings, listTemplates, createMeeting, startDualRecording, stopDualRecording, isRecording as checkIsRecording } from '$lib/api';
  import { Mic, Plus, Calendar, Clock, FileText, ChevronRight, Settings } from 'lucide-svelte';
  import { goto } from '$app/navigation';

  let newMeetingTitle = '';
  let showNewMeetingForm = false;
  let recordingInterval: ReturnType<typeof setInterval> | null = null;
  let recordingError: string | null = null;

  onMount(async () => {
    try {
      const [meetingList, templateList] = await Promise.all([
        listMeetings(),
        listTemplates()
      ]);
      meetings.set(meetingList);
      templates.set(templateList);
      
      // Check if currently recording
      const recording = await checkIsRecording();
      isRecording.set(recording);
      
      if (recording) {
        startRecordingTimer();
      }
    } catch (e) {
      console.error('Failed to load data:', e);
    }
  });

  function startRecordingTimer() {
    recordingTime.set(0);
    recordingInterval = setInterval(() => {
      recordingTime.update(t => t + 1);
    }, 1000);
  }

  function stopRecordingTimer() {
    if (recordingInterval) {
      clearInterval(recordingInterval);
      recordingInterval = null;
    }
  }

  async function handleCreateMeeting() {
    if (!newMeetingTitle.trim()) return;
    
    try {
      const meeting = await createMeeting(newMeetingTitle);
      meetings.update(m => [meeting, ...m]);
      newMeetingTitle = '';
      showNewMeetingForm = false;
      goto(`/meeting/${meeting.id}`);
    } catch (e) {
      console.error('Failed to create meeting:', e);
    }
  }

  async function handleStartRecording() {
    if (!newMeetingTitle.trim()) return;
    recordingError = null;

    try {
      const meeting = await createMeeting(newMeetingTitle);
      meetings.update(m => [meeting, ...m]);

      console.log('Starting dual recording for meeting:', meeting.id);
      await startDualRecording(meeting.id);
      console.log('Recording started successfully');

      newMeetingTitle = '';
      showNewMeetingForm = false;
      isRecording.set(true);
      startRecordingTimer();

      goto(`/meeting/${meeting.id}`);
    } catch (e) {
      console.error('Failed to start recording:', e);
      recordingError = String(e);
    }
  }

  async function handleStopRecording() {
    try {
      await stopDualRecording();
      isRecording.set(false);
      stopRecordingTimer();
    } catch (e) {
      console.error('Failed to stop recording:', e);
    }
  }

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    return date.toLocaleDateString('en-US', { 
      month: 'short', 
      day: 'numeric',
      year: 'numeric'
    });
  }

  function formatTime(dateStr: string): string {
    const date = new Date(dateStr);
    return date.toLocaleTimeString('en-US', { 
      hour: '2-digit', 
      minute: '2-digit' 
    });
  }

  function formatDuration(seconds?: number): string {
    if (!seconds) return '';
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  function formatRecordingTime(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }
</script>

<div class="flex h-screen bg-gray-900">
  <!-- Sidebar -->
  <aside class="w-80 bg-gray-800 border-r border-gray-700 flex flex-col">
    <!-- Header -->
    <div class="p-4 border-b border-gray-700">
      <div class="flex items-center justify-between mb-4">
        <div class="flex items-center gap-2">
          <div class="w-8 h-8 bg-blue-600 rounded-lg flex items-center justify-center">
            <FileText class="w-5 h-5 text-white" />
          </div>
          <h1 class="text-xl font-bold text-white">WizScribe</h1>
        </div>
        <a
          href="/settings"
          class="p-2 rounded-lg hover:bg-gray-700 text-gray-400 hover:text-white transition-colors"
          title="Settings"
        >
          <Settings class="w-5 h-5" />
        </a>
      </div>
      
      {#if $isRecording}
        <button
          on:click={handleStopRecording}
          class="w-full flex items-center justify-center gap-2 bg-red-600 hover:bg-red-700 text-white px-4 py-3 rounded-lg font-medium transition-colors"
        >
          <div class="w-3 h-3 bg-white rounded-sm"></div>
          Stop Recording ({$recordingTime > 0 ? formatRecordingTime($recordingTime) : '0:00'})
        </button>
      {:else}
        <button
          on:click={() => showNewMeetingForm = true}
          class="w-full flex items-center justify-center gap-2 bg-blue-600 hover:bg-blue-700 text-white px-4 py-3 rounded-lg font-medium transition-colors"
        >
          <Plus class="w-5 h-5" />
          New Meeting
        </button>
      {/if}
    </div>

    <!-- Meeting List -->
    <div class="flex-1 overflow-y-auto p-4">
      <h2 class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-3">Recent Meetings</h2>
      
      <div class="space-y-2">
        {#each $meetings as meeting}
          <a
            href="/meeting/{meeting.id}"
            class="block p-3 rounded-lg hover:bg-gray-700 transition-colors group"
          >
            <div class="flex items-start justify-between">
              <div class="flex-1 min-w-0">
                <h3 class="font-medium text-white truncate group-hover:text-blue-400 transition-colors">
                  {meeting.title}
                </h3>
                <div class="flex items-center gap-3 mt-1 text-sm text-gray-400">
                  <span class="flex items-center gap-1">
                    <Calendar class="w-3 h-3" />
                    {formatDate(meeting.created_at)}
                  </span>
                  {#if meeting.duration_secs}
                    <span class="flex items-center gap-1">
                      <Clock class="w-3 h-3" />
                      {formatDuration(meeting.duration_secs)}
                    </span>
                  {/if}
                </div>
              </div>
              <ChevronRight class="w-4 h-4 text-gray-500 group-hover:text-gray-300" />
            </div>
          </a>
        {:else}
          <div class="text-center py-8 text-gray-500">
            <FileText class="w-12 h-12 mx-auto mb-3 opacity-50" />
            <p>No meetings yet</p>
            <p class="text-sm mt-1">Create your first meeting to get started</p>
          </div>
        {/each}
      </div>
    </div>

    <!-- Templates -->
    <div class="p-4 border-t border-gray-700">
      <h2 class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-3">Templates</h2>
      <div class="flex flex-wrap gap-2">
        {#each $templates.slice(0, 4) as template}
          <button
            on:click={() => { newMeetingTitle = template.name; showNewMeetingForm = true; }}
            class="px-3 py-1.5 bg-gray-700 hover:bg-gray-600 rounded-full text-sm text-gray-300 transition-colors"
          >
            {template.name}
          </button>
        {/each}
      </div>
    </div>
  </aside>

  <!-- Main Content -->
  <main class="flex-1 flex items-center justify-center bg-gray-900">
    <div class="text-center max-w-md px-4">
      <div class="w-20 h-20 bg-gray-800 rounded-2xl flex items-center justify-center mx-auto mb-6">
        <Mic class="w-10 h-10 text-blue-500" />
      </div>
      <h2 class="text-2xl font-bold text-white mb-2">Welcome to WizScribe</h2>
      <p class="text-gray-400 mb-6">
        AI-powered meeting notes for Linux. Record, transcribe, and get AI summaries of your meetings.
      </p>
      <button
        on:click={() => showNewMeetingForm = true}
        class="bg-blue-600 hover:bg-blue-700 text-white px-6 py-3 rounded-lg font-medium transition-colors"
      >
        Start Your First Meeting
      </button>
    </div>
  </main>
</div>

<!-- New Meeting Modal -->
{#if showNewMeetingForm}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <div class="bg-gray-800 rounded-xl p-6 w-full max-w-md mx-4">
      <h3 class="text-lg font-semibold text-white mb-4">New Meeting</h3>
      
      <input
        type="text"
        bind:value={newMeetingTitle}
        placeholder="Meeting title..."
        class="w-full bg-gray-700 border border-gray-600 rounded-lg px-4 py-3 text-white placeholder-gray-400 focus:outline-none focus:border-blue-500 mb-4"
        on:keydown={(e) => e.key === 'Enter' && handleCreateMeeting()}
        autofocus
      />

      {#if recordingError}
        <div class="mb-4 p-3 bg-red-900/50 border border-red-700 rounded-lg text-red-300 text-sm">
          {recordingError}
        </div>
      {/if}
      
      <div class="flex gap-3">
        <button
          on:click={() => showNewMeetingForm = false}
          class="flex-1 bg-gray-700 hover:bg-gray-600 text-white px-4 py-2.5 rounded-lg font-medium transition-colors"
        >
          Cancel
        </button>
        <button
          on:click={handleCreateMeeting}
          disabled={!newMeetingTitle.trim()}
          class="flex-1 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white px-4 py-2.5 rounded-lg font-medium transition-colors"
        >
          Create
        </button>
        <button
          on:click={handleStartRecording}
          disabled={!newMeetingTitle.trim()}
          class="flex-1 bg-red-600 hover:bg-red-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white px-4 py-2.5 rounded-lg font-medium transition-colors flex items-center justify-center gap-2"
        >
          <Mic class="w-4 h-4" />
          Record
        </button>
      </div>
    </div>
  </div>
{/if}
