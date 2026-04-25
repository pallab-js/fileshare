<script lang="ts">
  import { Shield, Activity, Users, HardDrive, RefreshCw, X, CheckCircle2 } from 'lucide-svelte';
  import { peers, transfers, addToast } from '$lib/stores';
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { formatSize } from '$lib/utils';

  let stats = $state({
    activePeers: 0,
    totalTransferred: 0,
    networkLoad: '0 KB/s'
  });

  async function loadStats() {
    try {
      stats.activePeers = $peers.length;
      
      const history = await invoke<any[]>('get_history');
      const total = history
        .filter(h => h.status === 'completed')
        .reduce((acc, h) => acc + h.fileSize, 0);
      stats.totalTransferred = total;
    } catch (e) {
      console.error("Failed to load admin stats", e);
    }
  }

  async function cancelTransfer(id: string) {
    try {
      await invoke('cancel_transfer', { id });
      addToast('Transfer cancellation requested', 'info');
    } catch (e: any) {
      addToast(e.toString(), 'error');
    }
  }

  onMount(() => {
    loadStats();
    const interval = setInterval(loadStats, 5000);
    return () => clearInterval(interval);
  });

  let activeTransfers = $derived($transfers.filter(t => t.status === 'streaming'));
</script>

<div class="p-8 max-w-6xl mx-auto">
  <div class="flex items-center justify-between mb-8">
    <div class="flex items-center gap-4">
      <div class="w-12 h-12 bg-brand-translucent rounded-2xl flex items-center justify-center text-brand">
        <Shield size={28} />
      </div>
      <div>
        <h1 class="text-3xl font-medium tracking-tight mb-1 leading-none">Admin Dashboard</h1>
        <p class="text-text-secondary">Network-wide visibility and control.</p>
      </div>
    </div>
    <button 
      onclick={loadStats}
      class="flex items-center gap-2 px-4 py-2 bg-surface border border-border-card rounded-[6px] text-sm font-medium text-text-secondary hover:text-text-primary transition-all"
    >
      <RefreshCw size={16} />
      Refresh
    </button>
  </div>

  <!-- Stats Grid -->
  <div class="grid grid-cols-1 md:grid-cols-3 gap-6 mb-10">
    <div class="bg-surface border border-border-card rounded-2xl p-6">
      <div class="flex items-center gap-3 text-text-muted mb-4">
        <Users size={18} />
        <span class="text-xs font-mono uppercase tracking-widest">Active Peers</span>
      </div>
      <p class="text-4xl font-medium text-text-primary leading-none">{stats.activePeers}</p>
      <p class="text-xs text-text-muted mt-2">Devices currently reachable</p>
    </div>

    <div class="bg-surface border border-border-card rounded-2xl p-6">
      <div class="flex items-center gap-3 text-text-muted mb-4">
        <HardDrive size={18} />
        <span class="text-xs font-mono uppercase tracking-widest">Data Exchanged</span>
      </div>
      <p class="text-4xl font-medium text-text-primary leading-none">{formatSize(stats.totalTransferred)}</p>
      <p class="text-xs text-text-muted mt-2">Cumulative session volume</p>
    </div>

    <div class="bg-surface border border-border-card rounded-2xl p-6">
      <div class="flex items-center gap-3 text-text-muted mb-4">
        <Activity size={18} />
        <span class="text-xs font-mono uppercase tracking-widest">Network Load</span>
      </div>
      <p class="text-4xl font-medium text-text-primary leading-none">{stats.networkLoad}</p>
      <p class="text-xs text-text-muted mt-2">Current throughput estimation</p>
    </div>
  </div>

  <div class="grid grid-cols-1 lg:grid-cols-3 gap-8">
    <!-- Active Transfers -->
    <div class="lg:col-span-2 space-y-4">
      <h2 class="text-xl font-medium leading-none mb-4">Active Sockets</h2>
      {#each activeTransfers as t}
        <div class="bg-surface border border-border-card rounded-2xl p-6 animate-in fade-in duration-500">
          <div class="flex items-center justify-between mb-4">
            <div class="flex items-center gap-4">
              <div class="w-10 h-10 bg-background border border-border-card rounded-xl flex items-center justify-center text-brand">
                <RefreshCw size={20} class="animate-spin" />
              </div>
              <div>
                <p class="text-sm font-medium text-text-primary">{t.fileName}</p>
                <p class="text-xs text-text-muted">{t.sender} • {formatSize(t.fileSize)}</p>
              </div>
            </div>
            <button 
              onclick={() => cancelTransfer(t.id)}
              class="px-3 py-1 bg-danger/10 text-danger border border-danger/20 rounded-[6px] text-[10px] font-mono uppercase tracking-widest hover:bg-danger hover:text-surface transition-all"
            >
              Terminate
            </button>
          </div>
          
          <div class="space-y-2">
            <div class="flex justify-between text-[10px] font-mono text-text-muted uppercase">
              <span>Transfer Progress</span>
              <span class="text-brand font-medium">{Math.round(t.progress)}%</span>
            </div>
            <div class="h-1.5 w-full bg-background rounded-full overflow-hidden">
              <div class="h-full bg-brand transition-all duration-300" style="width: {t.progress}%"></div>
            </div>
          </div>
        </div>
      {:else}
        <div class="bg-surface/30 border border-border-card border-dashed rounded-[32px] py-16 flex flex-col items-center justify-center text-text-muted">
          <Activity size={48} class="mb-4 opacity-10" />
          <p class="text-sm">No active file streams detected.</p>
        </div>
      {/each}
    </div>

    <!-- Security Events -->
    <div class="space-y-4">
      <h2 class="text-xl font-medium leading-none mb-4">Audit Log</h2>
      <div class="bg-surface border border-border-card rounded-2xl p-6">
        <div class="space-y-4">
          {#each $transfers.slice(0, 10) as log}
            <div class="flex gap-3">
              <div class="mt-1 w-1.5 h-1.5 rounded-full {log.status === 'failed' || log.status === 'cancelled' || log.status === 'declined' ? 'bg-danger' : 'bg-brand'}"></div>
              <div>
                <p class="text-xs text-text-primary">{log.fileName} - {log.status}</p>
                <p class="text-[10px] text-text-muted font-mono uppercase mt-0.5">
                  {log.sender}
                </p>
              </div>
            </div>
          {:else}
            <p class="text-xs text-text-muted italic">No recent activity.</p>
          {/each}
        </div>
      </div>
    </div>
  </div>
</div>
