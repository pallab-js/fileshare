<script lang="ts">
  import { Monitor, Smartphone, Laptop, Server, Plus, Globe } from 'lucide-svelte';
  import { appState } from '$lib/stores.svelte';
  import { goto } from '$app/navigation';
  import { invoke } from '@tauri-apps/api/core';

  const icons = {
    mac: Laptop,
    windows: Monitor,
    linux: Server,
    mobile: Smartphone,
    unknown: Server
  };

  let showDirectConnect = $state(false);
  let manualIp = $state('');
  let manualPort = $state('50001');
  let isConnecting = $state(false);

  function getInitials(name: string) {
    return name.split(' ').map(n => n[0]).join('').toUpperCase().substring(0, 2);
  }

  function handleSend(peer: any) {
    goto(`/send?peerId=${encodeURIComponent(peer.id)}`);
  }

  async function handleDirectConnect() {
    if (!manualIp || !manualPort) return;
    isConnecting = true;
    try {
      await invoke('direct_connect', { ip: manualIp, port: parseInt(manualPort) });
      appState.addToast('Device added successfully', 'success');
      showDirectConnect = false;
      manualIp = '';
    } catch (e: any) {
      appState.addToast(e.toString(), 'error');
    } finally {
      isConnecting = false;
    }
  }
</script>

<div class="p-8">
  <div class="flex items-center justify-between mb-8">
    <div>
      <h1 class="text-3xl font-medium tracking-tight mb-2 leading-none">Devices</h1>
      <p class="text-text-secondary">Discovered devices on your local network.</p>
    </div>
    <div class="flex items-center gap-4">
      <button 
        onclick={() => showDirectConnect = true}
        class="flex items-center gap-2 px-4 py-2 bg-surface border border-border-card rounded-[6px] text-sm font-medium text-text-secondary hover:text-text-primary hover:border-text-muted transition-all active:scale-[0.98]"
      >
        <Plus size={18} />
        <span>Direct Connect</span>
      </button>

      <div class="flex items-center gap-2 px-3 py-1 bg-brand-translucent rounded-full border border-brand/20">
        <div class="w-2 h-2 bg-brand rounded-full {appState.peers.length === 0 ? 'animate-pulse' : ''}"></div>
        <span class="text-xs font-mono uppercase tracking-wider text-brand">
          {appState.peers.length === 0 ? 'Scanning...' : `${appState.peers.length} device${appState.peers.length !== 1 ? 's' : ''} found`}
        </span>
      </div>
    </div>
  </div>

  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
    {#each appState.peers as peer}
      {@const Icon = icons[peer.device_type as keyof typeof icons] || Server}
      <div class="group relative bg-surface border border-border-card rounded-2xl p-6 transition-all duration-300 hover:border-brand/40 hover:bg-surface/80">
        <div class="flex items-start justify-between mb-4">
          <div class="w-12 h-12 bg-background border border-border-card rounded-xl flex items-center justify-center text-lg font-medium text-brand group-hover:border-brand/30">
            {getInitials(peer.name)}
          </div>
          <div class="flex items-center gap-1.5">
            <div class="w-2 h-2 rounded-full bg-brand"></div>
            <span class="text-[10px] font-mono uppercase tracking-widest text-text-muted">online</span>
          </div>
        </div>
        
        <h3 class="text-lg font-medium text-text-primary mb-1 truncate">{peer.name}</h3>
        <p class="text-sm text-text-muted flex items-center gap-2">
          <Icon size={14} />
          {peer.device_type.toUpperCase()} • {peer.ip}
        </p>

        <button 
          onclick={() => handleSend(peer)}
          class="mt-6 w-full py-2 bg-background border border-border-card rounded-[6px] text-sm font-medium text-text-secondary transition-all hover:text-text-primary hover:border-text-secondary active:scale-[0.98]"
        >
          Send File
        </button>
      </div>
    {:else}
      <div class="col-span-full py-20 flex flex-col items-center justify-center text-text-muted border-2 border-dashed border-border-card rounded-[32px]">
        <Laptop size={48} class="mb-4 opacity-20" />
        <p>No devices found yet.</p>
        <p class="text-xs">Make sure others have DropBridge open.</p>
      </div>
    {/each}
  </div>
</div>

{#if showDirectConnect}
  <div class="fixed inset-0 bg-background/80 backdrop-blur-sm flex items-center justify-center z-50 p-4">
    <div class="bg-surface border border-border-card rounded-[32px] w-full max-sm overflow-hidden shadow-2xl animate-in zoom-in-95 duration-200">
      <div class="p-8">
        <div class="w-16 h-16 bg-brand-translucent rounded-2xl flex items-center justify-center mb-6 mx-auto">
          <Globe size={32} class="text-brand" />
        </div>
        
        <h2 class="text-2xl font-medium leading-none text-center mb-2">Direct Connect</h2>
        <p class="text-text-secondary text-center mb-8 text-sm">
          Connect manually via IP if mDNS discovery is blocked by your router.
        </p>

        <div class="space-y-4 mb-8">
          <div>
            <label class="block text-[10px] font-mono uppercase tracking-widest text-text-muted mb-1.5" for="ip">IP Address</label>
            <input 
              type="text" 
              id="ip"
              bind:value={manualIp}
              placeholder="e.g. 192.168.1.5"
              class="w-full bg-background border border-border-card rounded-[6px] px-4 py-2.5 text-sm focus:outline-none focus:border-brand/50 transition-colors"
            />
          </div>
          <div>
            <label class="block text-[10px] font-mono uppercase tracking-widest text-text-muted mb-1.5" for="port">Port</label>
            <input 
              type="text" 
              id="port"
              bind:value={manualPort}
              placeholder="50001"
              class="w-full bg-background border border-border-card rounded-[6px] px-4 py-2.5 text-sm focus:outline-none focus:border-brand/50 transition-colors"
            />
          </div>
        </div>

        <div class="flex gap-4">
          <button 
            onclick={() => showDirectConnect = false}
            class="flex-1 py-3 bg-surface border border-border-card rounded-full text-sm font-medium text-text-secondary hover:text-text-primary transition-all active:scale-[0.98]"
          >
            Cancel
          </button>
          <button 
            onclick={handleDirectConnect}
            disabled={isConnecting || !manualIp || !manualPort}
            class="flex-1 py-3 bg-brand text-surface rounded-full text-sm font-medium transition-all hover:bg-brand-hover active:scale-[0.98] shadow-lg shadow-brand/20 disabled:opacity-50"
          >
            {isConnecting ? 'Connecting...' : 'Connect'}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
