<script lang="ts">
  import { Shield, Activity, Users, StopCircle, RefreshCw } from 'lucide-svelte';

  // Mock data for admin dashboard
  let activeTransfers = $state([
    { id: '1', sender: "Alice", receiver: "Bob", file: 'Project_Assets.zip', progress: 65, speed: '45 MB/s' },
    { id: '2', sender: "Charlie", receiver: "Dave", file: 'Screen Recording.mov', progress: 12, speed: '12 MB/s' },
  ]);

  let stats = $state({
    activePeers: 12,
    totalTransferred: '45.2 GB',
    networkLoad: '12%',
  });
</script>

<div class="p-8 max-w-6xl mx-auto">
  <div class="flex items-center justify-between mb-10">
    <div class="flex items-center gap-4">
      <div class="w-12 h-12 bg-brand rounded-2xl flex items-center justify-center">
        <Shield size={24} class="text-surface" />
      </div>
      <div>
        <h1 class="text-3xl font-medium tracking-tight leading-none">Admin Dashboard</h1>
        <p class="text-text-secondary mt-2">Network-wide oversight and management.</p>
      </div>
    </div>
    <button class="flex items-center gap-2 px-4 py-2 bg-surface border border-border-card rounded-[6px] text-sm font-medium text-text-secondary hover:text-text-primary hover:border-text-secondary transition-all">
      <RefreshCw size={16} />
      Refresh Network
    </button>
  </div>

  <!-- Stats Grid -->
  <div class="grid grid-cols-1 md:grid-cols-3 gap-6 mb-10">
    <div class="bg-surface border border-border-card rounded-2xl p-6">
      <div class="flex items-center gap-3 text-text-muted mb-4">
        <Users size={18} />
        <span class="text-xs font-mono uppercase tracking-widest">Active Peers</span>
      </div>
      <p class="text-4xl font-medium text-brand">{stats.activePeers}</p>
    </div>
    <div class="bg-surface border border-border-card rounded-2xl p-6">
      <div class="flex items-center gap-3 text-text-muted mb-4">
        <Activity size={18} />
        <span class="text-xs font-mono uppercase tracking-widest">Total Shared Today</span>
      </div>
      <p class="text-4xl font-medium text-text-primary">{stats.totalTransferred}</p>
    </div>
    <div class="bg-surface border border-border-card rounded-2xl p-6">
      <div class="flex items-center gap-3 text-text-muted mb-4">
        <Activity size={18} />
        <span class="text-xs font-mono uppercase tracking-widest">LAN Load</span>
      </div>
      <div class="flex items-end gap-2">
        <p class="text-4xl font-medium text-text-primary">{stats.networkLoad}</p>
        <div class="w-full h-2 bg-background rounded-full mb-2 overflow-hidden">
          <div class="h-full bg-brand" style="width: {stats.networkLoad}"></div>
        </div>
      </div>
    </div>
  </div>

  <!-- Active Transfers Table -->
  <div class="bg-surface border border-border-card rounded-2xl overflow-hidden">
    <div class="px-6 py-4 border-b border-border-card bg-background/30 flex items-center justify-between">
      <h2 class="font-medium leading-none">Active Transfers</h2>
      <span class="px-2 py-0.5 bg-brand-translucent text-brand text-[10px] font-mono rounded uppercase">{activeTransfers.length} Live</span>
    </div>
    <table class="w-full text-left border-collapse">
      <thead>
        <tr class="border-b border-border-card bg-background/10">
          <th class="px-6 py-4 text-xs font-mono uppercase tracking-widest text-text-muted">Sender / Receiver</th>
          <th class="px-6 py-4 text-xs font-mono uppercase tracking-widest text-text-muted">File</th>
          <th class="px-6 py-4 text-xs font-mono uppercase tracking-widest text-text-muted">Progress</th>
          <th class="px-6 py-4 text-xs font-mono uppercase tracking-widest text-text-muted">Actions</th>
        </tr>
      </thead>
      <tbody class="divide-y divide-border-card/50">
        {#each activeTransfers as transfer}
          <tr class="hover:bg-background/20 transition-colors">
            <td class="px-6 py-4">
              <div class="flex flex-col">
                <span class="text-sm font-medium text-text-primary">{transfer.sender}</span>
                <span class="text-[10px] text-text-muted uppercase tracking-tighter">to {transfer.receiver}</span>
              </div>
            </td>
            <td class="px-6 py-4">
              <span class="text-sm text-text-secondary">{transfer.file}</span>
            </td>
            <td class="px-6 py-4">
              <div class="w-48">
                <div class="flex items-center justify-between mb-1.5">
                  <span class="text-[10px] font-mono text-text-muted uppercase tracking-wider">{transfer.speed}</span>
                  <span class="text-[10px] font-mono text-brand font-medium">{transfer.progress}%</span>
                </div>
                <div class="h-1.5 w-full bg-background rounded-full mb-2 overflow-hidden">
                  <div class="h-full bg-brand transition-all duration-500" style="width: {transfer.progress}%"></div>
                </div>
              </div>
            </td>
            <td class="px-6 py-4">
              <button class="p-2 text-text-muted hover:text-danger transition-colors group" title="Force Cancel">
                <StopCircle size={18} />
              </button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
