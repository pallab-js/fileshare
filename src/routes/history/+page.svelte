<script lang="ts">
  import { ArrowUpRight, ArrowDownLeft, CheckCircle2, XCircle, Clock, Loader2, Trash2, Filter, ChevronLeft, ChevronRight } from 'lucide-svelte';
  import { appState } from '$lib/stores.svelte';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { formatSize, formatSpeed, formatETA } from '$lib/utils';

  let historyItems = $state<any[]>([]);
  let filterStatus = $state<string>('all');
  let page = $state(0);
  const PAGE_SIZE = 50;

  let filteredItems = $derived(
    historyItems
      .filter(h => filterStatus === 'all' || h.status === filterStatus)
      .slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE)
  );

  let totalPages = $derived(
    Math.ceil(historyItems.filter(h => filterStatus === 'all' || h.status === filterStatus).length / PAGE_SIZE) || 1
  );

  async function loadHistory() {
    try {
      const history = await invoke<any[]>('get_history', { limit: 1000, offset: 0 });
      historyItems = history.map(h => ({
        id: h.id,
        fileName: h.fileName,
        fileSize: h.fileSize,
        sender: h.direction === 'sent' ? 'You → ' + h.peerName : h.peerName,
        progress: 100,
        status: h.status,
        timestamp: h.timestamp
      }));
    } catch (e) {
      console.error("Failed to load history", e);
    }
  }

  async function clearHistory() {
    if (confirm('Are you sure you want to clear your transfer history?')) {
      try {
        await invoke('clear_history');
        historyItems = [];
        appState.addToast('History cleared', 'success');
      } catch (e: any) {
        appState.addToast(e.toString(), 'error');
      }
    }
  }

  onMount(() => {
    loadHistory();
  });

  function getStatusClass(status: string) {
    switch (status) {
      case 'completed': return 'border-brand/40 text-brand';
      case 'failed':
      case 'cancelled': 
      case 'declined': return 'border-danger/40 text-danger';
      case 'streaming': return 'border-brand/40 text-brand bg-brand/5';
      default: return 'border-text-muted/40 text-text-muted';
    }
  }

  function getStatusIcon(status: string) {
    switch (status) {
      case 'completed': return CheckCircle2;
      case 'failed':
      case 'cancelled':
      case 'declined': return XCircle;
      case 'streaming': return Loader2;
      default: return Clock;
    }
  }

  function formatDate(timestamp: number) {
    if (!timestamp) return '--';
    return new Date(timestamp * 1000).toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  }
</script>

<div class="p-8 max-w-5xl mx-auto">
  <div class="mb-8 flex items-center justify-between">
    <div>
      <h1 class="text-3xl font-medium tracking-tight mb-2 leading-none">History</h1>
      <p class="text-text-secondary">Track your recent file transfers on this device.</p>
    </div>
    <div class="flex items-center gap-2">
      <button 
        onclick={clearHistory}
        class="p-2 text-text-muted hover:text-danger transition-colors flex items-center gap-2 text-sm font-medium"
      >
        <Trash2 size={18} />
        <span>Clear</span>
      </button>
      <button 
        onclick={loadHistory}
        class="p-2 text-text-muted hover:text-text-primary transition-colors"
      >
        <Clock size={20} />
      </button>
    </div>
  </div>

  <div class="mb-6 flex items-center justify-between">
    <div class="flex gap-2">
      {#each ['all', 'completed', 'failed', 'cancelled'] as status}
        <button
          onclick={() => { filterStatus = status; page = 0; }}
          class="px-4 py-1.5 rounded-full text-xs font-mono uppercase tracking-widest border transition-colors
            {filterStatus === status 
              ? 'bg-brand-translucent border-brand/40 text-brand' 
              : 'bg-surface border-border-card text-text-muted hover:text-text-primary hover:border-text-muted'}"
        >
          {status}
        </button>
      {/each}
    </div>
    
    {#if totalPages > 1}
      <div class="flex items-center gap-4 text-sm text-text-muted">
        <span>Page {page + 1} of {totalPages}</span>
        <div class="flex gap-1">
          <button 
            disabled={page === 0}
            onclick={() => page--}
            class="p-1 rounded-[6px] border border-border-card disabled:opacity-50 disabled:cursor-not-allowed hover:text-text-primary hover:border-text-muted transition-colors"
          >
            <ChevronLeft size={16} />
          </button>
          <button 
            disabled={page >= totalPages - 1}
            onclick={() => page++}
            class="p-1 rounded-[6px] border border-border-card disabled:opacity-50 disabled:cursor-not-allowed hover:text-text-primary hover:border-text-muted transition-colors"
          >
            <ChevronRight size={16} />
          </button>
        </div>
      </div>
    {/if}
  </div>

  <div class="bg-surface border border-border-card rounded-2xl overflow-hidden shadow-sm">
    <table class="w-full text-left border-collapse">
      <thead>
        <tr class="border-b border-border-card bg-background/50">
          <th class="px-6 py-4 text-xs font-mono uppercase tracking-widest text-text-muted">Time</th>
          <th class="px-6 py-4 text-xs font-mono uppercase tracking-widest text-text-muted">File</th>
          <th class="px-6 py-4 text-xs font-mono uppercase tracking-widest text-text-muted">Peer</th>
          <th class="px-6 py-4 text-xs font-mono uppercase tracking-widest text-text-muted">Status</th>
          <th class="px-6 py-4 text-xs font-mono uppercase tracking-widest text-text-muted text-right">Progress</th>
        </tr>
      </thead>
      <tbody class="divide-y divide-border-card/50">
        {#each filteredItems as t}
          {@const StatusIcon = getStatusIcon(t.status)}
          <tr class="hover:bg-background/30 transition-colors group">
            <td class="px-6 py-4 whitespace-nowrap">
              <span class="text-xs font-mono text-text-muted uppercase tracking-tighter">
                {formatDate(t.timestamp || 0)}
              </span>
            </td>
            <td class="px-6 py-4">
              <div class="max-w-[240px]">
                <p class="text-sm font-medium text-text-primary truncate">{t.fileName}</p>
                <p class="text-xs font-mono text-text-muted uppercase tracking-tighter">
                  {formatSize(t.fileSize)}
                </p>
              </div>
            </td>
            <td class="px-6 py-4">
              <span class="text-sm text-text-secondary">{t.sender}</span>
            </td>
            <td class="px-6 py-4">
              <div class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full border text-[10px] font-mono uppercase tracking-widest {getStatusClass(t.status)}">
                <StatusIcon size={10} class={t.status === 'streaming' ? 'animate-spin' : ''} />
                {t.status}
              </div>
            </td>
            <td class="px-6 py-4 text-right">
              {#if t.status === 'streaming'}
                <div class="flex flex-col items-end gap-1">
                  <span class="text-[10px] font-mono text-brand uppercase tracking-wider">
                    {formatSpeed(t.speedBps || 0)}
                  </span>
                  <div class="w-24 h-1 bg-background rounded-full overflow-hidden">
                    <div class="h-full bg-brand transition-all duration-300" style="width: {t.progress}%"></div>
                  </div>
                </div>
              {:else if t.status === 'completed'}
                <span class="text-[9px] font-mono text-brand uppercase tracking-wider">100%</span>
              {:else}
                <span class="text-[9px] font-mono text-text-muted uppercase tracking-wider">--</span>
              {/if}
            </td>
          </tr>
        {:else}
          <tr>
            <td colspan="5" class="px-6 py-20 text-center text-text-muted italic">
              No transfer history found.
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
