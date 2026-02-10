<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { ArrowLeft, Key, Plus, Trash2, Check, Mic, Users } from 'lucide-svelte';
  import { goto } from '$app/navigation';
  import type { Template, AudioDevice, DualAudioConfig } from '$lib/types';
  import {
    listAudioDevices,
    setRecordingDevice,
    getSelectedAudioDevice,
    getAudioDevicesByType,
    getDualAudioConfig,
    setDualAudioConfig
  } from '$lib/api';

  let apiKey = '';
  let provider: 'openai' | 'anthropic' = 'openai';
  let templates: Template[] = [];
  let newTemplateName = '';
  let newTemplateStructure = '';
  let hasApiKey = false;
  let saving = false;

  // Audio device state (legacy single device)
  let audioDevices: AudioDevice[] = [];
  let selectedDeviceIndex: number | null = null;
  let loadingDevices = false;
  let savingDevice = false;

  // Dual audio device state
  let microphoneDevices: AudioDevice[] = [];
  let monitorDevices: AudioDevice[] = [];
  let selectedMicIndex: number | null = null;
  let selectedSystemIndex: number | null = null;
  let loadingDualDevices = false;
  let savingDualConfig = false;

  onMount(async () => {
    await Promise.all([loadTemplates(), loadAiConfig(), loadAudioDevices(), loadDualAudioDevices()]);
  });

  async function loadAiConfig() {
    try {
      const config = await invoke('get_ai_config') as { has_api_key: boolean; provider: string };
      hasApiKey = config.has_api_key;
      if (config.provider === 'anthropic') {
        provider = 'anthropic';
      } else {
        provider = 'openai';
      }
    } catch (e) {
      console.error('Failed to load AI config:', e);
    }
  }

  async function loadAudioDevices() {
    loadingDevices = true;
    try {
      audioDevices = await listAudioDevices();
      selectedDeviceIndex = await getSelectedAudioDevice();
    } catch (e) {
      console.error('Failed to load audio devices:', e);
    } finally {
      loadingDevices = false;
    }
  }

  async function loadDualAudioDevices() {
    loadingDualDevices = true;
    try {
      const [mics, monitors, config] = await Promise.all([
        getAudioDevicesByType('microphone'),
        getAudioDevicesByType('monitor'),
        getDualAudioConfig()
      ]);
      microphoneDevices = mics;
      monitorDevices = monitors;
      selectedMicIndex = config.mic_device_index;
      selectedSystemIndex = config.system_device_index;
    } catch (e) {
      console.error('Failed to load dual audio devices:', e);
    } finally {
      loadingDualDevices = false;
    }
  }

  async function handleDeviceChange(event: Event) {
    const select = event.target as HTMLSelectElement;
    const value = select.value;
    const deviceIndex = value === 'default' ? null : parseInt(value, 10);

    savingDevice = true;
    try {
      await setRecordingDevice(deviceIndex);
      selectedDeviceIndex = deviceIndex;
    } catch (e) {
      console.error('Failed to set audio device:', e);
      alert('Failed to set audio device: ' + e);
    } finally {
      savingDevice = false;
    }
  }

  async function handleMicChange(event: Event) {
    const select = event.target as HTMLSelectElement;
    const value = select.value;
    const deviceIndex = value === 'auto' ? null : parseInt(value, 10);

    savingDualConfig = true;
    try {
      await setDualAudioConfig(deviceIndex, selectedSystemIndex);
      selectedMicIndex = deviceIndex;
    } catch (e) {
      console.error('Failed to set microphone device:', e);
      alert('Failed to set microphone device: ' + e);
    } finally {
      savingDualConfig = false;
    }
  }

  async function handleSystemChange(event: Event) {
    const select = event.target as HTMLSelectElement;
    const value = select.value;
    const deviceIndex = value === 'auto' ? null : parseInt(value, 10);

    savingDualConfig = true;
    try {
      await setDualAudioConfig(selectedMicIndex, deviceIndex);
      selectedSystemIndex = deviceIndex;
    } catch (e) {
      console.error('Failed to set system audio device:', e);
      alert('Failed to set system audio device: ' + e);
    } finally {
      savingDualConfig = false;
    }
  }

  async function loadTemplates() {
    try {
      templates = await invoke('list_templates');
    } catch (e) {
      console.error('Failed to load templates:', e);
    }
  }

  async function saveApiKey() {
    if (!apiKey.trim()) return;

    saving = true;
    try {
      await invoke('set_ai_api_key', {
        apiKey: apiKey.trim(),
        provider: provider,
      });
      hasApiKey = true;
      apiKey = '';
      alert('API key saved successfully!');
    } catch (e) {
      console.error('Failed to save API key:', e);
      alert('Failed to save API key: ' + e);
    } finally {
      saving = false;
    }
  }

  async function createTemplate() {
    if (!newTemplateName.trim()) return;

    try {
      const structure = newTemplateStructure.trim() || '[{"type":"heading","content":"Notes"},{"type":"bullet","content":""}]';
      await invoke('create_template', {
        name: newTemplateName.trim(),
        structureJson: structure,
      });
      newTemplateName = '';
      newTemplateStructure = '';
      await loadTemplates();
    } catch (e) {
      console.error('Failed to create template:', e);
    }
  }

  async function deleteTemplate(id: string) {
    if (!confirm('Are you sure you want to delete this template?')) return;

    try {
      await invoke('delete_template', { id });
      await loadTemplates();
    } catch (e) {
      console.error('Failed to delete template:', e);
    }
  }

  function formatDate(dateStr: string): string {
    return new Date(dateStr).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    });
  }
</script>

<div class="max-w-3xl mx-auto p-6">
  <div class="flex items-center gap-4 mb-8">
    <button class="p-2 rounded-lg hover:bg-gray-700 text-gray-400 hover:text-white transition-colors" on:click={() => goto('/')}>
      <ArrowLeft class="w-5 h-5" />
    </button>
    <h1 class="text-2xl font-bold text-white">Settings</h1>
  </div>

  <!-- AI Configuration -->
  <div class="bg-gray-800 rounded-xl border border-gray-700 p-6 mb-6">
    <div class="flex items-center gap-3 mb-6">
      <div class="w-10 h-10 bg-blue-600 rounded-lg flex items-center justify-center">
        <Key class="w-5 h-5 text-white" />
      </div>
      <div>
        <h2 class="text-lg font-medium text-white">AI Configuration</h2>
        <p class="text-sm text-gray-400">Configure your AI provider for summaries and chat</p>
      </div>
      {#if hasApiKey}
        <div class="ml-auto flex items-center gap-2 text-green-400 text-sm">
          <Check class="w-4 h-4" />
          <span>Configured</span>
        </div>
      {/if}
    </div>

    <div class="space-y-4">
      <div>
        <label for="provider-select" class="block text-sm font-medium text-gray-300 mb-2">Provider</label>
        <select id="provider-select" bind:value={provider} class="w-full px-4 py-2.5 rounded-lg border border-gray-600 bg-gray-700 text-white focus:outline-none focus:ring-2 focus:ring-blue-500">
          <option value="openai">OpenAI (GPT-4)</option>
          <option value="anthropic">Anthropic (Claude 3)</option>
        </select>
      </div>

      <div>
        <label for="api-key-input" class="block text-sm font-medium text-gray-300 mb-2">
          API Key {#if hasApiKey}<span class="text-gray-500">(enter new key to update)</span>{/if}
        </label>
        <input
          id="api-key-input"
          type="password"
          bind:value={apiKey}
          placeholder={hasApiKey ? '••••••••••••••••' : 'Enter your API key'}
          class="w-full px-4 py-2.5 rounded-lg border border-gray-600 bg-gray-700 text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
        <p class="text-xs text-gray-500 mt-2">
          Your API key is stored locally and never sent to our servers.
        </p>
      </div>

      <button
        class="w-full sm:w-auto px-6 py-2.5 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        on:click={saveApiKey}
        disabled={saving || !apiKey.trim()}
      >
        {saving ? 'Saving...' : 'Save API Key'}
      </button>
    </div>
  </div>

  <!-- Dual Audio Configuration -->
  <div class="bg-gray-800 rounded-xl border border-gray-700 p-6 mb-6">
    <div class="flex items-center gap-3 mb-6">
      <div class="w-10 h-10 bg-green-600 rounded-lg flex items-center justify-center">
        <Mic class="w-5 h-5 text-white" />
      </div>
      <div>
        <h2 class="text-lg font-medium text-white">Audio Sources</h2>
        <p class="text-sm text-gray-400">Configure microphone and system audio for speaker identification</p>
      </div>
    </div>

    <div class="space-y-6">
      <!-- Microphone Selection -->
      <div>
        <label for="mic-device-select" class="block text-sm font-medium text-gray-300 mb-2">
          <span class="flex items-center gap-2">
            <Mic class="w-4 h-4" />
            Microphone (Your voice - "Me")
          </span>
        </label>
        {#if loadingDualDevices}
          <div class="text-gray-400 text-sm">Loading devices...</div>
        {:else}
          <select
            id="mic-device-select"
            value={selectedMicIndex === null ? 'auto' : selectedMicIndex.toString()}
            on:change={handleMicChange}
            disabled={savingDualConfig}
            class="w-full px-4 py-2.5 rounded-lg border border-gray-600 bg-gray-700 text-white focus:outline-none focus:ring-2 focus:ring-green-500 disabled:opacity-50"
          >
            <option value="auto">Auto-detect (first available)</option>
            {#each microphoneDevices as device}
              <option value={device.index.toString()}>
                {device.name}
              </option>
            {/each}
          </select>
        {/if}
        <p class="text-xs text-gray-500 mt-2">
          Select your microphone input device. Speech from this source will be labeled as "Me" in transcripts.
        </p>
      </div>

      <!-- System Audio Selection -->
      <div>
        <label for="system-device-select" class="block text-sm font-medium text-gray-300 mb-2">
          <span class="flex items-center gap-2">
            <Users class="w-4 h-4" />
            System Audio (Meeting participants - "Them")
          </span>
        </label>
        {#if loadingDualDevices}
          <div class="text-gray-400 text-sm">Loading devices...</div>
        {:else if monitorDevices.length === 0}
          <div class="text-amber-400 text-sm bg-amber-400/10 px-3 py-2 rounded-lg">
            No monitor sources available. Make sure PulseAudio or PipeWire is configured correctly.
          </div>
        {:else}
          <select
            id="system-device-select"
            value={selectedSystemIndex === null ? 'auto' : selectedSystemIndex.toString()}
            on:change={handleSystemChange}
            disabled={savingDualConfig}
            class="w-full px-4 py-2.5 rounded-lg border border-gray-600 bg-gray-700 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50"
          >
            <option value="auto">Auto-detect (first monitor)</option>
            {#each monitorDevices as device}
              <option value={device.index.toString()}>
                {device.name}
              </option>
            {/each}
          </select>
        {/if}
        <p class="text-xs text-gray-500 mt-2">
          Select a monitor source to capture audio from video calls (Teams, Zoom, Meet). Speech from this source will be labeled as "Them".
        </p>
      </div>

      <button
        class="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white text-sm font-medium rounded-lg transition-colors"
        on:click={loadDualAudioDevices}
        disabled={loadingDualDevices}
      >
        {loadingDualDevices ? 'Refreshing...' : 'Refresh Devices'}
      </button>
    </div>
  </div>

  <!-- Note Templates -->
  <div class="bg-gray-800 rounded-xl border border-gray-700 p-6">
    <h2 class="text-lg font-medium text-white mb-6">Note Templates</h2>

    <div class="space-y-4 mb-6">
      <div>
        <label for="template-name" class="block text-sm font-medium text-gray-300 mb-2">Template Name</label>
        <input
          id="template-name"
          type="text"
          bind:value={newTemplateName}
          placeholder="e.g., Sprint Review"
          class="w-full px-4 py-2.5 rounded-lg border border-gray-600 bg-gray-700 text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>
      <div>
        <label for="template-structure" class="block text-sm font-medium text-gray-300 mb-2">Structure (JSON)</label>
        <textarea
          id="template-structure"
          bind:value={newTemplateStructure}
          placeholder={'[{"type":"heading","content":"Agenda"},{"type":"bullet","content":""}]'}
          class="w-full px-4 py-2.5 rounded-lg border border-gray-600 bg-gray-700 text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono text-sm"
          rows="4"
        ></textarea>
        <p class="text-xs text-gray-500 mt-2">
          Use types: heading, bullet, checkbox
        </p>
      </div>
      <button
        class="flex items-center gap-2 px-4 py-2.5 bg-gray-700 hover:bg-gray-600 text-white font-medium rounded-lg transition-colors"
        on:click={createTemplate}
      >
        <Plus class="w-4 h-4" />
        Add Template
      </button>
    </div>

    {#if templates.length > 0}
      <div class="border-t border-gray-700 pt-4">
        {#each templates as template}
          <div class="py-3 flex items-center justify-between">
            <div>
              <h4 class="font-medium text-white">{template.name}</h4>
              <p class="text-xs text-gray-500">ID: {template.id}</p>
            </div>
            <button
              class="p-2 rounded-lg text-red-400 hover:bg-red-900/30 transition-colors"
              on:click={() => deleteTemplate(template.id)}
            >
              <Trash2 class="w-4 h-4" />
            </button>
          </div>
        {/each}
      </div>
    {:else}
      <p class="text-gray-500 text-sm">No custom templates yet.</p>
    {/if}
  </div>
</div>
