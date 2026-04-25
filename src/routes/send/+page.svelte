<script lang="ts">
  import { File, X, UploadCloud, Laptop } from 'lucide-svelte';
  import { appState } from '$lib/stores.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { page } from '$app/stores';
  import { formatSize, formatSpeed, formatETA } from '$lib/utils';
  import { open } from '@tauri-apps/plugin-dialog';

  let selectedPeerId = $state('');
  let files = $state<{ name: string, size: number, path: string }[]>([]);
  let isDragging = $state(false);

  $effect(() => {
    const id = $page.url.searchParams.get('peerId');
    if (id) selectedPeerId = id;
  });

  function getInitials(name: string) {
    return name.split(' ').map(n => n[0]).join('').toUpperCase().substring(0, 2);
  }

  async function browseFiles() {
    const selected = await open({ multiple: true, directory: false });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    for (const p of paths) {
      try {
        const [name, size] = await invoke<[string, number]>('get_file_meta', { path: p });
        files = [...files, { name, size, path: p }];
      } catch (e: any) {
        appState.addToast(e.toString(), 'error');
      }
    }
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    isDragging = false;
    const droppedFiles = Array.from(e.dataTransfer?.files ?? []);
    files = [
      ...files,
      ...droppedFiles.map(f => ({
        name: f.name,
        size: f.size,
        path: (f as any).path ?? f.name
      }))
    ];
  }

  async function sendFiles() {
    const peer = appState.peers.find(p => p.id === selectedPeerId);
    if (!peer || files.length === 0) return;

    for (const file of files) {
      try {
        const id = await invoke<string>('send_file', {
          filePath: file.path,
          recipientIp: peer.ip,
          port: peer.port
        });

        appState.addTransfer({
          id,
          fileName: file.name,
          fileSize: file.size,
          sender: 'You',
          progress: 0,
          status: 'pending'
        });
      } catch (e: any) {
        appState.addToast(e.toString(), 'error');
      }
    }
    files = [];
  }
</script>

<div class="p-8 max-w-4xl mx-auto">
  <h1 class="text-3xl font-medium tracking-tight mb-8 leading-none">Send Files</h1>

  <!-- Recipient Selection -->
  <div class="mb-10">
    <label class="block text-xs font-mono uppercase tracking-widest text-text-muted mb-4" for="recipient">Select Recipient</label>
    <div class="flex gap-3 overflow-x-auto pb-2 scrollbar-hide">
      {#each appState.peers as peer}
        <button
          onclick={() => selectedPeerId = peer.id}
          class="flex-shrink-0 flex items-center gap-3 px-4 py-3 rounded-[6px] border transition-all duration-200
            {selectedPeerId === peer.id 
              ? 'bg-brand-translucent border-brand/40 text-brand' 
              : 'bg-surface border-border-card text-text-secondary hover:border-text-muted'}"
        >
          <div class="w-8 h-8 rounded-[6px] bg-background border border-current/20 flex items-center justify-center text-xs font-medium">
            {getInitials(peer.name)}
          </div>
          <span class="text-sm font-medium whitespace-nowrap">{peer.name}</span>
        </button>
      {:else}
        <p class="text-sm text-text-muted italic">No devices available.</p>
      {/each}
    </div>
  </div>

  <!-- Dropzone -->
  <div
    role="button"
    tabindex="0"
    onclick={browseFiles}
    onkeydown={(e) => e.key === 'Enter' && browseFiles()}
    ondragover={(e) => { e.preventDefault(); isDragging = true; }}
    ondragleave={() => isDragging = false}
    ondrop={handleDrop}
    class="relative group cursor-pointer mb-10"
  >
    <div class="absolute inset-0 bg-brand/5 rounded-[32px] blur-2xl opacity-0 group-hover:opacity-100 transition-opacity duration-500"></div>
    <div
      class="relative h-64 border-2 border-dashed rounded-[32px] flex flex-col items-center justify-center transition-all duration-300
        {isDragging ? 'border-brand bg-brand/5 scale-[1.01]' : 'border-border-card bg-surface hover:border-text-muted'}"
    >
      <div class="w-16 h-16 bg-background border border-border-card rounded-2xl flex items-center justify-center mb-4 transition-transform duration-300 group-hover:-translate-y-1">
        <UploadCloud size={32} class={isDragging ? 'text-brand' : 'text-text-muted'} />
      </div>
      <p class="text-lg font-medium text-text-primary mb-1">Drop files or click to browse</p>
      <p class="text-sm text-text-muted">Files will be sent to the selected device</p>
    </div>
  </div>

  <!-- Queue / Active Transfers -->
  <div class="space-y-3 mb-10">
    {#if files.length > 0 || appState.transfers.some(t => t.sender === 'You' && (t.status === 'streaming' || t.status === 'pending'))}
      <label class="block text-xs font-mono uppercase tracking-widest text-text-muted mb-2" for="queue">Queue</label>
      
      {#each files as file, i}
        <div class="flex items-center justify-between p-4 bg-surface border border-border-card rounded-[6px] group transition-all hover:border-text-muted">
          <div class="flex items-center gap-4">
            <div class="w-10 h-10 bg-background border border-border-card rounded-[6px] flex items-center justify-center">
              <File size={20} class="text-text-muted" />
            </div>
            <div>
              <p class="text-sm font-medium text-text-primary truncate max-w-[300px]">{file.name}</p>
              <p class="text-xs font-mono text-text-muted uppercase">{file.size > 0 ? formatSize(file.size) : 'Ready to send'}</p>
            </div>
          </div>
          <button 
            onclick={() => files = files.filter((_, idx) => idx !== i)}
            class="p-2 text-text-muted hover:text-danger transition-colors"
          >
            <X size={18} />
          </button>
        </div>
      {/each}

      {#each appState.transfers.filter(t => t.sender === 'You' && (t.status === 'streaming' || t.status === 'pending')) as t}
        <div class="flex items-center justify-between p-4 bg-surface border border-brand/40 rounded-[6px] group transition-all">
          <div class="flex items-center gap-4">
            <div class="w-10 h-10 bg-background border border-brand/20 rounded-[6px] flex items-center justify-center">
              <File size={20} class="text-brand" />
            </div>
            <div>
              <p class="text-sm font-medium text-text-primary truncate max-w-[300px]">{t.fileName}</p>
              {#if t.status === 'streaming'}
                <p class="text-xs font-mono text-brand uppercase">
                  {formatSpeed(t.speedBps ?? 0)} · ETA {formatETA(t.fileSize * (1 - t.progress/100), t.speedBps ?? 0)}
                </p>
              {:else}
                <p class="text-xs font-mono text-text-muted uppercase">Pending...</p>
              {/if}
            </div>
          </div>
          <div class="w-24">
             <div class="h-1.5 w-full bg-background rounded-full overflow-hidden">
               <div class="h-full bg-brand transition-all duration-300" style="width: {t.progress}%"></div>
             </div>
          </div>
        </div>
      {/each}
    {/if}
  </div>

  <div class="flex justify-end">
    <button 
      onclick={sendFiles}
      disabled={!selectedPeerId || files.length === 0}
      class="px-8 py-3 bg-brand text-surface font-medium rounded-full transition-all hover:bg-brand-hover hover:scale-[1.02] active:scale-[0.98] shadow-lg shadow-brand/20 disabled:opacity-50 disabled:cursor-not-allowed disabled:scale-100"
    >
      Send to {appState.peers.find(r => r.id === selectedPeerId)?.name || 'Device'}
    </button>
  </div>
</div>

<style>
  .scrollbar-hide::-webkit-scrollbar {
    display: none;
  }
  .scrollbar-hide {
    -ms-overflow-style: none;
    scrollbar-width: none;
  }
</style>
