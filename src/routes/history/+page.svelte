<script lang="ts">
  import { ArrowUpRight, ArrowDownLeft, CheckCircle2, XCircle, Clock, Loader2 } from 'lucide-svelte';
  import { transfers } from '$lib/stores';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  async function loadHistory() {
    try {
      const history = await invoke<any[]>('get_history');
      // Map DB history to store format
      const mapped = history.map(h => ({
        id: h.id,
        fileName: h.fileName,
        fileSize: h.fileSize,
        sender: h.direction === 'sent' ? 'You' : h.peerName,
        progress: 100,
        status: h.status
      }));
      transfers.set(mapped);
    } catch (e) {
      console.error("Failed to load history", e);
    }
  }

  onMount(() => {
    loadHistory();
  });

  function getStatusClass(status: string) {
    switch (status) {
      case 'completed': return 'border-brand/40 text-brand';
      case 'cancelled': 
      case 'declined': return 'border-danger/40 text-danger';
      case 'streaming': return 'border-brand/40 text-brand bg-brand/5';
      default: return 'border-text-muted/40 text-text-muted';
    }
  }

  function getStatusIcon(status: string) {
    switch (status) {
      case 'completed': return CheckCircle2;
      case 'cancelled':
      case 'declined': return XCircle;
      case 'streaming': return Loader2;
      default: return Clock;
    }
  }
</script>

<div class="p-8 max-w-5xl mx-auto">
  <div class="mb-8 flex items-center justify-between">
    <div>
      <h1 class="text-3xl font-medium tracking-tight mb-2 leading-none">History</h1>
      <p class="text-text-secondary">Track your recent file transfers on this device.</p>
    </div>
    <button 
      onclick={loadHistory}
      class="p-2 text-text-muted hover:text-text-primary transition-colors"
    >
      <Clock size={20} />
    </button>
  </div>

  <div class="bg-surface border border-border-card rounded-2xl overflow-hidden">
    <table class="w-full text-left border-collapse">
      <thead>
        <tr class="border-b border-border-card bg-background/50">
          <th class="px-6 py-4 text-xs font-mono uppercase tracking-widest text-text-muted">Type</th>
          <th class="px-6 py-4 text-xs font-mono uppercase tracking-widest text-text-muted">File</th>
          <th class="px-6 py-4 text-xs font-mono uppercase tracking-widest text-text-muted">Peer</th>
          <th class="px-6 py-4 text-xs font-mono uppercase tracking-widest text-text-muted">Status</th>
          <th class="px-6 py-4 text-xs font-mono uppercase tracking-widest text-text-muted">Progress</th>
        </tr>
      </thead>
      <tbody class="divide-y divide-border-card/50">
        {#each $transfers as t}
          <tr class="hover:bg-background/30 transition-colors group">
            <td class="px-6 py-4">
              <div class="flex items-center gap-2">
                {#if t.sender === 'You'}
                  <div class="w-8 h-8 rounded-[6px] bg-brand-translucent flex items-center justify-center text-brand">
                    <ArrowUpRight size={16} />
                  </div>
                {:else}
                  <div class="w-8 h-8 rounded-[6px] bg-text-muted/10 flex items-center justify-center text-text-muted">
                    <ArrowDownLeft size={16} />
                  </div>
                {/if}
              </div>
            </td>
            <td class="px-6 py-4">
              <div class="max-w-[240px]">
                <p class="text-sm font-medium text-text-primary truncate">{t.fileName}</p>
                <p class="text-xs font-mono text-text-muted uppercase tracking-tighter">
                  {(t.fileSize / (1024 * 1024)).toFixed(2)} MB
                </p>
              </div>
            </td>
            <td class="px-6 py-4">
              <span class="text-sm text-text-secondary">{t.sender}</span>
            </td>
            <td class="px-6 py-4">
              <div class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full border text-[10px] font-mono uppercase tracking-widest {getStatusClass(t.status)}">
                <svelte:component this={getStatusIcon(t.status)} size={10} class={t.status === 'streaming' ? 'animate-spin' : ''} />
                {t.status}
              </div>
            </td>
            <td class="px-6 py-4">
              {#if t.status === 'streaming'}
                <div class="w-24">
                  <div class="h-1 w-full bg-background rounded-full overflow-hidden">
                    <div class="h-full bg-brand transition-all duration-300" style="width: {t.progress}%"></div>
                  </div>
                  <span class="text-[9px] font-mono text-text-muted uppercase tracking-wider">{Math.round(t.progress)}%</span>
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
