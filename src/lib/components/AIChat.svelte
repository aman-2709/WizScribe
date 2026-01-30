<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { Send, Bot, User, Loader2, Settings } from 'lucide-svelte';
  import type { Meeting } from '$lib/types';

  export let meeting: Meeting;

  interface ChatMessage {
    role: 'user' | 'assistant';
    content: string;
    timestamp: number;
  }

  let messages: ChatMessage[] = [];
  let inputMessage = '';
  let isLoading = false;
  let error: string | null = null;
  let showSettings = false;
  let apiKey = '';
  let provider: 'openai' | 'anthropic' = 'openai';

  onMount(() => {
    // Add welcome message
    messages = [{
      role: 'assistant',
      content: 'Hello! I can help you analyze this meeting transcript. Ask me anything about the discussion, decisions, or action items.',
      timestamp: Date.now(),
    }];
  });

  async function sendMessage() {
    if (!inputMessage.trim() || isLoading) return;

    const userMessage = inputMessage.trim();
    inputMessage = '';
    
    messages = [...messages, {
      role: 'user',
      content: userMessage,
      timestamp: Date.now(),
    }];

    isLoading = true;
    error = null;

    try {
      if (!meeting.transcript) {
        throw new Error('No transcript available. Please transcribe the meeting first.');
      }

      const response: string = await invoke('chat_with_ai', {
        transcript: meeting.transcript,
        question: userMessage,
      });

      messages = [...messages, {
        role: 'assistant',
        content: response,
        timestamp: Date.now(),
      }];
    } catch (e) {
      error = String(e);
      console.error('Failed to get AI response:', e);
      
      messages = [...messages, {
        role: 'assistant',
        content: `Error: ${error}. Please make sure you have configured your AI API key in settings.`,
        timestamp: Date.now(),
      }];
    } finally {
      isLoading = false;
    }
  }

  async function saveApiKey() {
    if (!apiKey.trim()) return;

    try {
      await invoke('set_ai_api_key', {
        apiKey: apiKey.trim(),
        provider: provider,
      });
      
      showSettings = false;
      apiKey = '';
      error = null;
    } catch (e) {
      error = String(e);
      console.error('Failed to set API key:', e);
    }
  }

  function formatTime(timestamp: number): string {
    return new Date(timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  }
</script>

<div class="flex flex-col h-full">
  <div class="flex items-center justify-between mb-4">
    <h3 class="text-lg font-medium text-surface-800 flex items-center gap-2">
      <Bot class="w-5 h-5" />
      AI Assistant
    </h3>
    <button
      class="btn-ghost p-2"
      on:click={() => showSettings = !showSettings}
      title="AI Settings"
    >
      <Settings class="w-4 h-4" />
    </button>
  </div>

  {#if showSettings}
    <div class="bg-surface-100 rounded-lg p-4 mb-4">
      <h4 class="text-sm font-medium text-surface-700 mb-3">AI Configuration</h4>
      <div class="space-y-3">
        <div>
          <label class="block text-sm text-surface-600 mb-1">Provider</label>
          <select bind:value={provider} class="input text-sm">
            <option value="openai">OpenAI (GPT-4)</option>
            <option value="anthropic">Anthropic (Claude)</option>
          </select>
        </div>
        <div>
          <label class="block text-sm text-surface-600 mb-1">API Key</label>
          <input
            type="password"
            bind:value={apiKey}
            placeholder="Enter your API key"
            class="input text-sm"
          />
        </div>
        {#if error}
          <div class="text-red-500 text-sm">{error}</div>
        {/if}
        <div class="flex gap-2">
          <button class="btn-primary text-sm flex-1" on:click={saveApiKey}>
            Save
          </button>
          <button class="btn-secondary text-sm" on:click={() => showSettings = false}>
            Cancel
          </button>
        </div>
      </div>
    </div>
  {/if}

  <div class="flex-1 overflow-y-auto scrollbar-thin space-y-4 mb-4 pr-2">
    {#each messages as message}
      <div class="flex gap-3 {message.role === 'user' ? 'flex-row-reverse' : ''}">
        <div class="flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center {message.role === 'user' ? 'bg-primary-100 text-primary-600' : 'bg-surface-200 text-surface-600'}">
          {#if message.role === 'user'}
            <User class="w-4 h-4" />
          {:else}
            <Bot class="w-4 h-4" />
          {/if}
        </div>
        <div class="flex-1 {message.role === 'user' ? 'text-right' : ''}">
          <div class="inline-block max-w-[85%] {message.role === 'user' ? 'bg-primary-500 text-white' : 'bg-surface-100 text-surface-800'} rounded-2xl px-4 py-2 text-sm">
            <div class="whitespace-pre-wrap">{message.content}</div>
          </div>
          <div class="text-xs text-surface-400 mt-1">
            {formatTime(message.timestamp)}
          </div>
        </div>
      </div>
    {/each}
    
    {#if isLoading}
      <div class="flex gap-3">
        <div class="flex-shrink-0 w-8 h-8 rounded-full bg-surface-200 text-surface-600 flex items-center justify-center">
          <Bot class="w-4 h-4" />
        </div>
        <div class="bg-surface-100 rounded-2xl px-4 py-2">
          <Loader2 class="w-4 h-4 animate-spin text-surface-400" />
        </div>
      </div>
    {/if}
  </div>

  <div class="flex gap-2">
    <textarea
      bind:value={inputMessage}
      on:keydown={handleKeyDown}
      placeholder="Ask about the meeting..."
      class="textarea flex-1 text-sm min-h-[44px] max-h-32"
      rows="1"
    ></textarea>
    <button
      class="btn-primary px-4"
      on:click={sendMessage}
      disabled={!inputMessage.trim() || isLoading}
    >
      <Send class="w-4 h-4" />
    </button>
  </div>
</div>
