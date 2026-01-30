<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { Loader2, Save, Clock, Type, List, CheckSquare, Heading } from 'lucide-svelte';
  import type { Note, Meeting, NoteTimestamp, Template } from '$lib/types';
  import { currentNote } from '$lib/stores';

  export let meeting: Meeting;

  let content = '';
  let timestamps: NoteTimestamp[] = [];
  let isSaving = false;
  let error: string | null = null;
  let templates: Template[] = [];
  let selectedTemplate: string | null = null;
  let saveTimeout: ReturnType<typeof setTimeout>;

  onMount(async () => {
    await loadNote();
    await loadTemplates();
  });

  onDestroy(() => {
    if (saveTimeout) clearTimeout(saveTimeout);
    // Auto-save on destroy
    saveNote();
  });

  async function loadNote() {
    try {
      const note: Note | null = await invoke('get_note', {
        meetingId: meeting.id,
      });
      
      if (note) {
        content = note.content;
        timestamps = JSON.parse(note.timestamps_json);
        currentNote.set(note);
      }
    } catch (e) {
      console.error('Failed to load note:', e);
    }
  }

  async function loadTemplates() {
    try {
      templates = await invoke('get_templates');
    } catch (e) {
      console.error('Failed to load templates:', e);
    }
  }

  async function saveNote() {
    if (isSaving) return;
    
    isSaving = true;
    try {
      const note: Note = await invoke('save_note', {
        meetingId: meeting.id,
        content,
        timestampsJson: JSON.stringify(timestamps),
      });
      currentNote.set(note);
    } catch (e) {
      console.error('Failed to save note:', e);
      error = String(e);
    } finally {
      isSaving = false;
    }
  }

  function handleInput() {
    // Debounced auto-save
    if (saveTimeout) clearTimeout(saveTimeout);
    saveTimeout = setTimeout(saveNote, 1000);
  }

  function insertTimestamp() {
    const id = crypto.randomUUID();
    const timestamp: NoteTimestamp = {
      id,
      time: Date.now(),
      label: new Date().toLocaleTimeString(),
    };
    timestamps = [...timestamps, timestamp];
    
    // Insert timestamp marker in content
    const cursorPosition = getCursorPosition();
    const timestampText = `[${timestamp.label}] `;
    content = content.slice(0, cursorPosition) + timestampText + content.slice(cursorPosition);
    
    handleInput();
  }

  function getCursorPosition(): number {
    const textarea = document.querySelector('textarea') as HTMLTextAreaElement;
    return textarea?.selectionStart || content.length;
  }

  function applyTemplate(templateId: string) {
    const template = templates.find(t => t.id === templateId);
    if (!template) return;

    try {
      const structure: Array<{ type: string; content: string }> = JSON.parse(template.structure_json);
      let newContent = '';
      
      for (const item of structure) {
        switch (item.type) {
          case 'heading':
            newContent += `## ${item.content}\n\n`;
            break;
          case 'bullet':
            newContent += `• ${item.content}\n`;
            break;
          case 'checkbox':
            newContent += `- [ ] ${item.content}\n`;
            break;
        }
      }
      
      content = newContent;
      handleInput();
      selectedTemplate = null;
    } catch (e) {
      console.error('Failed to apply template:', e);
    }
  }

  function formatText(type: 'bold' | 'italic' | 'heading' | 'bullet' | 'checkbox') {
    const textarea = document.querySelector('textarea') as HTMLTextAreaElement;
    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    const selectedText = content.slice(start, end);
    let formattedText = '';
    let cursorOffset = 0;

    switch (type) {
      case 'bold':
        formattedText = `**${selectedText || 'bold text'}**`;
        cursorOffset = selectedText ? 0 : 2;
        break;
      case 'italic':
        formattedText = `*${selectedText || 'italic text'}*`;
        cursorOffset = selectedText ? 0 : 1;
        break;
      case 'heading':
        formattedText = `\n### ${selectedText || 'Heading'}\n`;
        cursorOffset = selectedText ? 0 : 0;
        break;
      case 'bullet':
        formattedText = `\n• ${selectedText || 'Item'}\n`;
        cursorOffset = selectedText ? 0 : 0;
        break;
      case 'checkbox':
        formattedText = `\n- [ ] ${selectedText || 'Task'}\n`;
        cursorOffset = selectedText ? 0 : 0;
        break;
    }

    content = content.slice(0, start) + formattedText + content.slice(end);
    
    // Restore cursor position
    setTimeout(() => {
      textarea.focus();
      const newPosition = start + formattedText.length - cursorOffset;
      textarea.setSelectionRange(newPosition, newPosition);
    }, 0);

    handleInput();
  }
</script>

<div class="flex flex-col h-full">
  <div class="flex items-center justify-between mb-3">
    <h3 class="text-lg font-medium text-surface-800">Notes</h3>
    <div class="flex items-center gap-2">
      {#if templates.length > 0}
        <select
          class="input text-sm py-1 w-40"
          bind:value={selectedTemplate}
          on:change={() => selectedTemplate && applyTemplate(selectedTemplate)}
        >
          <option value={null}>Apply Template...</option>
          {#each templates as template}
            <option value={template.id}>{template.name}</option>
          {/each}
        </select>
      {/if}
      
      <button
        class="btn-ghost text-sm"
        on:click={insertTimestamp}
        title="Insert Timestamp"
      >
        <Clock class="w-4 h-4" />
      </button>
      
      <button
        class="btn-primary text-sm"
        on:click={saveNote}
        disabled={isSaving}
      >
        {#if isSaving}
          <Loader2 class="w-4 h-4 animate-spin" />
        {:else}
          <Save class="w-4 h-4" />
        {/if}
        Save
      </button>
    </div>
  </div>

  <div class="flex items-center gap-1 p-2 bg-surface-100 rounded-lg mb-3">
    <button
      class="p-1.5 rounded hover:bg-surface-200 text-surface-600"
      on:click={() => formatText('heading')}
      title="Heading"
    >
      <Heading class="w-4 h-4" />
    </button>
    <button
      class="p-1.5 rounded hover:bg-surface-200 text-surface-600 font-bold"
      on:click={() => formatText('bold')}
      title="Bold"
    >
      B
    </button>
    <button
      class="p-1.5 rounded hover:bg-surface-200 text-surface-600 italic"
      on:click={() => formatText('italic')}
      title="Italic"
    >
      I
    </button>
    <div class="w-px h-5 bg-surface-300 mx-1"></div>
    <button
      class="p-1.5 rounded hover:bg-surface-200 text-surface-600"
      on:click={() => formatText('bullet')}
      title="Bullet List"
    >
      <List class="w-4 h-4" />
    </button>
    <button
      class="p-1.5 rounded hover:bg-surface-200 text-surface-600"
      on:click={() => formatText('checkbox')}
      title="Checkbox"
    >
      <CheckSquare class="w-4 h-4" />
    </button>
  </div>

  {#if error}
    <div class="bg-red-50 text-red-600 p-3 rounded-lg mb-3 text-sm">
      {error}
    </div>
  {/if}

  <textarea
    class="textarea flex-1 font-mono text-sm leading-relaxed"
    bind:value={content}
    on:input={handleInput}
    placeholder="Take your notes here...&#10;&#10;Use markdown formatting for rich text.&#10;- [ ] Create action items&#10;**Bold text** and *italic text*&#10;## Headings"
  ></textarea>
</div>
