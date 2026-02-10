<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/stores';
  import { currentMeeting, currentNote, isRecording, recordingTime, isTranscribing, isGeneratingSummary } from '$lib/stores';
  import { getMeeting, getNote, updateNote, startDualRecording, stopDualRecording, isRecording as checkIsRecording, transcribeMeeting, generateSummary, askQuestion, draftFollowupEmail, exportToMarkdown } from '$lib/api';
  import type { Meeting, MeetingSummary, ChatMessage } from '$lib/types';
  import { Mic, Square, Play, Pause, FileText, Sparkles, Download, Send, Bot, User, Loader2, ArrowLeft } from 'lucide-svelte';
  import { goto } from '$app/navigation';

  const meetingId = $page.params.id;
  
  let noteContent = '';
  let transcriptContent = '';
  let summary: MeetingSummary | null = null;
  let chatMessages: ChatMessage[] = [];
  let chatInput = '';
  let audioElement: HTMLAudioElement | null = null;
  let isPlaying = false;
  let currentTime = 0;
  let showAiChat = false;
  let emailRecipient = '';
  let showEmailModal = false;
  let draftedEmail = '';
  let recordingInterval: ReturnType<typeof setInterval> | null = null;
  let recordingError: string | null = null;

  onMount(async () => {
    try {
      const [meeting, note] = await Promise.all([
        getMeeting(meetingId),
        getNote(meetingId)
      ]);
      
      if (meeting) {
        currentMeeting.set(meeting);
        noteContent = note?.content || '';
        transcriptContent = meeting.transcript || '';
        if (meeting.summary) {
          try {
            summary = JSON.parse(meeting.summary);
          } catch {
            summary = null;
          }
        }
        
        // Check recording status
        const recording = await checkIsRecording();
        isRecording.set(recording);
        if (recording) {
          startRecordingTimer();
        }
      } else {
        goto('/');
      }
    } catch (e) {
      console.error('Failed to load meeting:', e);
    }

    // Auto-save notes every 5 seconds
    const autoSave = setInterval(() => {
      if ($currentNote && noteContent !== $currentNote.content) {
        handleSaveNote();
      }
    }, 5000);

    return () => {
      clearInterval(autoSave);
      stopRecordingTimer();
    };
  });

  onDestroy(() => {
    stopRecordingTimer();
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

  async function handleSaveNote() {
    try {
      // Extract timestamps from note content (format: [MM:SS] or [HH:MM:SS])
      const timestampRegex = /\[(\d{1,2}:)?(\d{1,2}):(\d{2})\]/g;
      const timestamps: number[] = [];
      let match;
      
      while ((match = timestampRegex.exec(noteContent)) !== null) {
        const hours = match[1] ? parseInt(match[1].replace(':', '')) : 0;
        const minutes = parseInt(match[2]);
        const seconds = parseInt(match[3]);
        timestamps.push(hours * 3600 + minutes * 60 + seconds);
      }
      
      await updateNote(meetingId, noteContent, timestamps);
    } catch (e) {
      console.error('Failed to save note:', e);
    }
  }

  async function handleStartRecording() {
    recordingError = null;
    try {
      console.log('Starting dual recording for meeting:', meetingId);
      const status = await startDualRecording(meetingId);
      console.log('Recording started:', status);
      isRecording.set(true);
      startRecordingTimer();
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

      // Reload meeting to get updated audio path
      const meeting = await getMeeting(meetingId);
      if (meeting) {
        currentMeeting.set(meeting);
      }
    } catch (e) {
      console.error('Failed to stop recording:', e);
    }
  }

  async function handleTranscribe() {
    isTranscribing.set(true);
    try {
      const transcript = await transcribeMeeting(meetingId);
      transcriptContent = transcript;
      
      // Reload meeting
      const meeting = await getMeeting(meetingId);
      if (meeting) {
        currentMeeting.set(meeting);
      }
    } catch (e) {
      console.error('Failed to transcribe:', e);
    } finally {
      isTranscribing.set(false);
    }
  }

  async function handleGenerateSummary() {
    if (!transcriptContent) {
      console.error('No transcript available for summary');
      return;
    }

    isGeneratingSummary.set(true);
    try {
      const result = await generateSummary(meetingId, transcriptContent);
      // Parse the result into a MeetingSummary structure
      summary = {
        summary: result,
        action_items: [],
        key_decisions: []
      };

      // Reload meeting
      const meeting = await getMeeting(meetingId);
      if (meeting) {
        currentMeeting.set(meeting);
      }
    } catch (e) {
      console.error('Failed to generate summary:', e);
    } finally {
      isGeneratingSummary.set(false);
    }
  }

  async function handleSendMessage() {
    if (!chatInput.trim() || !transcriptContent) return;

    const question = chatInput;
    chatMessages = [...chatMessages, { role: 'user', content: question }];
    chatInput = '';

    try {
      const response = await askQuestion(transcriptContent, question);
      chatMessages = [...chatMessages, { role: 'assistant', content: response }];
    } catch (e) {
      console.error('Failed to get answer:', e);
      chatMessages = [...chatMessages, { role: 'assistant', content: 'Sorry, I encountered an error. Please make sure AI is configured in settings.' }];
    }
  }

  async function handleDraftEmail() {
    if (!emailRecipient.trim() || !transcriptContent) return;

    try {
      const summaryText = summary?.summary || '';
      const email = await draftFollowupEmail(transcriptContent, summaryText);
      // Prepend recipient info to the drafted email
      draftedEmail = `To: ${emailRecipient}\n\n${email}`;
    } catch (e) {
      console.error('Failed to draft email:', e);
      draftedEmail = 'Error: ' + (e as Error).message;
    }
  }

  async function handleExport() {
    try {
      await exportToMarkdown(meetingId);
    } catch (e) {
      console.error('Failed to export:', e);
    }
  }

  function insertTimestamp() {
    const now = currentTime || 0;
    const mins = Math.floor(now / 60);
    const secs = Math.floor(now % 60);
    const timestamp = `[${mins}:${secs.toString().padStart(2, '0')}] `;
    
    const textarea = document.getElementById('note-editor') as HTMLTextAreaElement;
    if (textarea) {
      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      noteContent = noteContent.substring(0, start) + timestamp + noteContent.substring(end);
      
      // Save immediately after inserting timestamp
      setTimeout(handleSaveNote, 100);
    }
  }

  function seekToTimestamp(timestamp: string) {
    // Parse [MM:SS] or [HH:MM:SS] format
    const match = timestamp.match(/\[(\d{1,2}:)?(\d{1,2}):(\d{2})\]/);
    if (match && audioElement) {
      const hours = match[1] ? parseInt(match[1].replace(':', '')) : 0;
      const minutes = parseInt(match[2]);
      const seconds = parseInt(match[3]);
      const totalSeconds = hours * 3600 + minutes * 60 + seconds;
      
      audioElement.currentTime = totalSeconds;
      audioElement.play();
      isPlaying = true;
    }
  }

  function formatTime(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  function formatRecordingTime(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    return date.toLocaleDateString('en-US', { 
      weekday: 'long',
      month: 'long', 
      day: 'numeric',
      year: 'numeric'
    });
  }
</script>

<div class="flex h-screen bg-gray-900">
  <!-- Sidebar -->
  <aside class="w-80 bg-gray-800 border-r border-gray-700 flex flex-col">
    <div class="p-4 border-b border-gray-700">
      <a href="/" class="flex items-center gap-2 text-gray-400 hover:text-white transition-colors mb-4">
        <ArrowLeft class="w-4 h-4" />
        Back to Meetings
      </a>
      
      <h1 class="text-lg font-bold text-white truncate">{$currentMeeting?.title || 'Meeting'}</h1>
      <p class="text-sm text-gray-400">{$currentMeeting ? formatDate($currentMeeting.created_at) : ''}</p>
      
      <div class="flex gap-2 mt-4">
        {#if $isRecording}
          <button
            on:click={handleStopRecording}
            class="flex-1 flex items-center justify-center gap-2 bg-red-600 hover:bg-red-700 text-white px-3 py-2 rounded-lg text-sm font-medium transition-colors"
          >
            <Square class="w-4 h-4" />
            Stop ({$recordingTime > 0 ? formatRecordingTime($recordingTime) : '0:00'})
          </button>
        {:else if $currentMeeting?.audio_path}
          <button
            on:click={() => audioElement?.paused ? audioElement?.play() : audioElement?.pause()}
            class="flex items-center justify-center gap-2 bg-gray-700 hover:bg-gray-600 text-white px-3 py-2 rounded-lg text-sm font-medium transition-colors"
          >
            {#if isPlaying}
              <Pause class="w-4 h-4" />
            {:else}
              <Play class="w-4 h-4" />
            {/if}
          </button>
        {:else}
          <button
            on:click={handleStartRecording}
            class="flex-1 flex items-center justify-center gap-2 bg-red-600 hover:bg-red-700 text-white px-3 py-2 rounded-lg text-sm font-medium transition-colors"
          >
            <Mic class="w-4 h-4" />
            Record
          </button>
        {/if}
        
        <button
          on:click={handleExport}
          class="flex items-center justify-center gap-2 bg-gray-700 hover:bg-gray-600 text-white px-3 py-2 rounded-lg text-sm font-medium transition-colors"
          title="Export to Markdown"
        >
          <Download class="w-4 h-4" />
        </button>
      </div>

      {#if recordingError}
        <div class="mt-2 p-2 bg-red-900/50 border border-red-700 rounded-lg text-red-300 text-xs">
          {recordingError}
        </div>
      {/if}
    </div>

    <!-- AI Actions -->
    <div class="p-4 border-b border-gray-700 space-y-2">
      {#if !$currentMeeting?.transcript}
        <button
          on:click={handleTranscribe}
          disabled={!$currentMeeting?.audio_path || $isTranscribing}
          class="w-full flex items-center justify-center gap-2 bg-purple-600 hover:bg-purple-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white px-3 py-2 rounded-lg text-sm font-medium transition-colors"
        >
          {#if $isTranscribing}
            <Loader2 class="w-4 h-4 animate-spin" />
            Transcribing...
          {:else}
            <FileText class="w-4 h-4" />
            Transcribe Audio
          {/if}
        </button>
      {/if}
      
      {#if $currentMeeting?.transcript && !summary}
        <button
          on:click={handleGenerateSummary}
          disabled={$isGeneratingSummary}
          class="w-full flex items-center justify-center gap-2 bg-green-600 hover:bg-green-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white px-3 py-2 rounded-lg text-sm font-medium transition-colors"
        >
          {#if $isGeneratingSummary}
            <Loader2 class="w-4 h-4 animate-spin" />
            Generating...
          {:else}
            <Sparkles class="w-4 h-4" />
            Generate Summary
          {/if}
        </button>
      {/if}
      
      {#if $currentMeeting?.transcript}
        <button
          on:click={() => showAiChat = !showAiChat}
          class="w-full flex items-center justify-center gap-2 bg-blue-600 hover:bg-blue-700 text-white px-3 py-2 rounded-lg text-sm font-medium transition-colors"
        >
          <Bot class="w-4 h-4" />
          {showAiChat ? 'Hide AI Chat' : 'Ask AI About Meeting'}
        </button>
        
        <button
          on:click={() => showEmailModal = true}
          class="w-full flex items-center justify-center gap-2 bg-gray-700 hover:bg-gray-600 text-white px-3 py-2 rounded-lg text-sm font-medium transition-colors"
        >
          Draft Follow-up Email
        </button>
      {/if}
    </div>

    <!-- Summary Panel -->
    {#if summary}
      <div class="flex-1 overflow-y-auto p-4">
        <h3 class="text-sm font-semibold text-gray-300 mb-3">AI Summary</h3>
        
        <div class="space-y-4 text-sm">
          <div>
            <h4 class="text-xs font-medium text-gray-400 uppercase tracking-wider mb-1">Summary</h4>
            <p class="text-gray-300 leading-relaxed">{summary.summary}</p>
          </div>
          
          {#if summary.action_items.length > 0}
            <div>
              <h4 class="text-xs font-medium text-gray-400 uppercase tracking-wider mb-1">Action Items</h4>
              <ul class="space-y-1">
                {#each summary.action_items as item}
                  <li class="text-gray-300 flex items-start gap-2">
                    <span class="text-blue-400 mt-0.5">•</span>
                    <span>{item}</span>
                  </li>
                {/each}
              </ul>
            </div>
          {/if}
          
          {#if summary.key_decisions.length > 0}
            <div>
              <h4 class="text-xs font-medium text-gray-400 uppercase tracking-wider mb-1">Key Decisions</h4>
              <ul class="space-y-1">
                {#each summary.key_decisions as decision}
                  <li class="text-gray-300 flex items-start gap-2">
                    <span class="text-green-400 mt-0.5">✓</span>
                    <span>{decision}</span>
                  </li>
                {/each}
              </ul>
            </div>
          {/if}
        </div>
      </div>
    {:else}
      <div class="flex-1"></div>
    {/if}
  </aside>

  <!-- Main Content -->
  <main class="flex-1 flex flex-col min-w-0">
    <!-- Audio Player -->
    {#if $currentMeeting?.audio_path}
      <div class="bg-gray-800 border-b border-gray-700 p-3 flex items-center gap-4">
        <button
          on:click={() => {
            if (audioElement?.paused) {
              audioElement?.play();
              isPlaying = true;
            } else {
              audioElement?.pause();
              isPlaying = false;
            }
          }}
          class="w-10 h-10 bg-blue-600 hover:bg-blue-700 rounded-full flex items-center justify-center text-white transition-colors"
        >
          {#if isPlaying}
            <Pause class="w-5 h-5" />
          {:else}
            <Play class="w-5 h-5 ml-0.5" />
          {/if}
        </button>
        
        <div class="flex-1">
          <input
            type="range"
            min="0"
            max={audioElement?.duration || 100}
            value={currentTime}
            on:input={(e) => {
              if (audioElement) {
                audioElement.currentTime = Number(e.currentTarget.value);
              }
            }}
            class="w-full h-1.5 bg-gray-600 rounded-lg appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-3 [&::-webkit-slider-thumb]:bg-blue-500 [&::-webkit-slider-thumb]:rounded-full"
          />
          <div class="flex justify-between text-xs text-gray-400 mt-1">
            <span>{formatTime(currentTime)}</span>
            <span>{formatTime(audioElement?.duration || 0)}</span>
          </div>
        </div>
        
        <audio
          bind:this={audioElement}
          src="{$currentMeeting.audio_path}"
          on:timeupdate={() => currentTime = audioElement?.currentTime || 0}
          on:ended={() => isPlaying = false}
          class="hidden"
        ></audio>
      </div>
    {/if}

    <!-- Split View -->
    <div class="flex-1 flex min-h-0">
      <!-- Notes Editor -->
      <div class="flex-1 flex flex-col border-r border-gray-700 min-w-0">
        <div class="bg-gray-800 border-b border-gray-700 px-4 py-2 flex items-center justify-between">
          <h2 class="text-sm font-semibold text-gray-300">Notes</h2>
          <button
            on:click={insertTimestamp}
            class="text-xs bg-gray-700 hover:bg-gray-600 text-gray-300 px-2 py-1 rounded transition-colors"
            title="Insert timestamp (Ctrl+T)"
          >
            + Timestamp
          </button>
        </div>
        <textarea
          id="note-editor"
          bind:value={noteContent}
          on:input={handleSaveNote}
          on:keydown={(e) => {
            if ((e.ctrlKey || e.metaKey) && e.key === 't') {
              e.preventDefault();
              insertTimestamp();
            }
          }}
          placeholder="Take notes here... Use Ctrl+T to insert timestamps."
          class="flex-1 bg-gray-900 text-gray-200 p-4 resize-none focus:outline-none font-mono text-sm leading-relaxed"
        ></textarea>
      </div>

      <!-- Transcript -->
      <div class="flex-1 flex flex-col min-w-0 {$currentMeeting?.transcript ? '' : 'hidden'}">
        <div class="bg-gray-800 border-b border-gray-700 px-4 py-2">
          <h2 class="text-sm font-semibold text-gray-300">Transcript</h2>
        </div>
        <div class="flex-1 overflow-y-auto p-4 bg-gray-900">
          {#if transcriptContent}
            <div class="space-y-2 text-sm">
              {#each transcriptContent.split('\n') as line}
                {#if line.trim()}
                  <div class="flex gap-3">
                    <button
                      on:click={() => seekToTimestamp(line)}
                      class="text-blue-400 hover:text-blue-300 font-mono text-xs shrink-0 mt-0.5"
                    >
                      {line.match(/\[.*?\]/)?.[0] || ''}
                    </button>
                    <span class="text-gray-300">{line.replace(/\[.*?\]\s*/, '')}</span>
                  </div>
                {/if}
              {/each}
            </div>
          {:else}
            <div class="text-center text-gray-500 py-8">
              <FileText class="w-12 h-12 mx-auto mb-3 opacity-50" />
              <p>No transcript yet</p>
              <p class="text-sm mt-1">Transcribe the audio to see it here</p>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </main>

  <!-- AI Chat Panel -->
  {#if showAiChat}
    <div class="w-96 bg-gray-800 border-l border-gray-700 flex flex-col">
      <div class="p-4 border-b border-gray-700 flex items-center justify-between">
        <h3 class="font-semibold text-white flex items-center gap-2">
          <Bot class="w-5 h-5 text-blue-400" />
          Ask About Meeting
        </h3>
        <button
          on:click={() => showAiChat = false}
          class="text-gray-400 hover:text-white"
        >
          ×
        </button>
      </div>
      
      <div class="flex-1 overflow-y-auto p-4 space-y-4">
        {#each chatMessages as message}
          <div class="flex gap-3 {message.role === 'user' ? 'flex-row-reverse' : ''}">
            <div class="w-8 h-8 rounded-full bg-gray-700 flex items-center justify-center shrink-0">
              {#if message.role === 'user'}
                <User class="w-4 h-4 text-gray-300" />
              {:else}
                <Bot class="w-4 h-4 text-blue-400" />
              {/if}
            </div>
            <div class="flex-1 {message.role === 'user' ? 'text-right' : ''}">
              <div class="inline-block bg-{message.role === 'user' ? 'blue' : 'gray'}-700 rounded-lg px-3 py-2 text-sm text-gray-200 max-w-full text-left">
                {message.content}
              </div>
            </div>
          </div>
        {/each}
      </div>
      
      <div class="p-4 border-t border-gray-700">
        <div class="flex gap-2">
          <input
            type="text"
            bind:value={chatInput}
            placeholder="Ask a question..."
            on:keydown={(e) => e.key === 'Enter' && handleSendMessage()}
            class="flex-1 bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white placeholder-gray-400 focus:outline-none focus:border-blue-500 text-sm"
          />
          <button
            on:click={handleSendMessage}
            disabled={!chatInput.trim()}
            class="bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white px-3 py-2 rounded-lg transition-colors"
          >
            <Send class="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<!-- Email Modal -->
{#if showEmailModal}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <div class="bg-gray-800 rounded-xl p-6 w-full max-w-2xl mx-4 max-h-[90vh] overflow-y-auto">
      <h3 class="text-lg font-semibold text-white mb-4">Draft Follow-up Email</h3>
      
      {#if !draftedEmail}
        <div class="mb-4">
          <label class="block text-sm text-gray-400 mb-1">Recipient</label>
          <input
            type="text"
            bind:value={emailRecipient}
            placeholder="recipient@example.com"
            class="w-full bg-gray-700 border border-gray-600 rounded-lg px-4 py-3 text-white placeholder-gray-400 focus:outline-none focus:border-blue-500"
            on:keydown={(e) => e.key === 'Enter' && handleDraftEmail()}
          />
        </div>
        
        <div class="flex gap-3">
          <button
            on:click={() => showEmailModal = false}
            class="flex-1 bg-gray-700 hover:bg-gray-600 text-white px-4 py-2.5 rounded-lg font-medium transition-colors"
          >
            Cancel
          </button>
          <button
            on:click={handleDraftEmail}
            disabled={!emailRecipient.trim()}
            class="flex-1 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white px-4 py-2.5 rounded-lg font-medium transition-colors"
          >
            Draft Email
          </button>
        </div>
      {:else}
        <textarea
          bind:value={draftedEmail}
          class="w-full h-96 bg-gray-700 border border-gray-600 rounded-lg px-4 py-3 text-white text-sm font-mono focus:outline-none focus:border-blue-500 mb-4 resize-none"
        ></textarea>
        
        <div class="flex gap-3">
          <button
            on:click={() => { draftedEmail = ''; emailRecipient = ''; }}
            class="flex-1 bg-gray-700 hover:bg-gray-600 text-white px-4 py-2.5 rounded-lg font-medium transition-colors"
          >
            Back
          </button>
          <button
            on:click={() => {
              navigator.clipboard.writeText(draftedEmail);
              showEmailModal = false;
            }}
            class="flex-1 bg-blue-600 hover:bg-blue-700 text-white px-4 py-2.5 rounded-lg font-medium transition-colors"
          >
            Copy to Clipboard
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}
