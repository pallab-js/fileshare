<script lang="ts">
  import { Monitor, Smartphone, Laptop, Server } from 'lucide-svelte';
  import { peers } from '$lib/stores';
  import { goto } from '$app/navigation';

  function getIcon(type: string) {
    if (type === 'mac') return Laptop;
    if (type === 'windows') return Monitor;
    if (type === 'linux') return Server;
    if (type === 'mobile') return Smartphone;
    return Server;
  }

  function getInitials(name: string) {
    return name.split(' ').map(n => n[0]).join('').toUpperCase().substring(0, 2);
  }

  function handleSend(peer: any) {
    goto(`/send?peerId=${encodeURIComponent(peer.id)}`);
  }
</script>

<div class="p-8">
  <div class="flex items-center justify-between mb-8">
    <div>
      <h1 class="text-3xl font-medium tracking-tight mb-2 leading-none">Devices</h1>
      <p class="text-text-secondary">Discovered devices on your local network.</p>
    </div>
    <div class="flex items-center gap-2 px-3 py-1 bg-brand-translucent rounded-full border border-brand/20">
      <div class="w-2 h-2 bg-brand rounded-full {$peers.length === 0 ? 'animate-pulse' : ''}"></div>
      <span class="text-xs font-mono uppercase tracking-wider text-brand">
        {$peers.length === 0 ? 'Scanning...' : `${$peers.length} device${$peers.length !== 1 ? 's' : ''} found`}
      </span>
    </div>
  </div>

  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
    {#each $peers as peer}
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
          <svelte:component this={getIcon(peer.device_type)} size={14} />
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
