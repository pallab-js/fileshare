<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { Laptop, Send, History, Settings, File, Check, X } from 'lucide-svelte';
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { sendNotification } from '@tauri-apps/plugin-notification';
  import { appState } from '$lib/stores.svelte';
  import { type Peer, type Transfer, type TransferRequestPayload, type TransferProgressPayload } from '$lib/types';
  import { getVersion } from '@tauri-apps/api/app';
  import { formatSize, formatSpeed } from '$lib/utils';

  let { children } = $props();

  const navItems = [
    { name: 'Devices', icon: Laptop, path: '/' },
    { name: 'Send', icon: Send, path: '/send' },
    { name: 'History', icon: History, path: '/history' },
    { name: 'Settings', icon: Settings, path: '/settings' },
  ];

  let version = $state('');
  let notificationsEnabled = $state(true);

  onMount(() => {
    getVersion().then(v => version = v);

    invoke<Record<string, string>>('get_settings').then(s => {
      if (s.notifications) notificationsEnabled = s.notifications === 'true';
    }).catch(console.error);

    invoke<Peer[]>('discover_peers').then(p => appState.setPeers(p));

    const unlistenPeer = listen<Peer>('peer_discovered', (event) => {
      appState.updatePeer(event.payload);
    });

    const unlistenPeerRemoved = listen<string>('peer_removed', (event) => {
      appState.removePeer(event.payload);
    });

    const unlistenRequest = listen<TransferRequestPayload>('transfer_requested', (event) => {
      const transfer: Transfer = {
        id: event.payload.id,
        fileName: event.payload.file_name,
        fileSize: event.payload.file_size,
        sender: event.payload.sender,
        progress: 0,
        status: 'pending'
      };
      appState.setPending(transfer);
      appState.addToast(`Incoming transfer from ${transfer.sender}`, 'info');
      if (notificationsEnabled) {
        sendNotification({
          title: 'DropBridge — Incoming File',
          body: `${transfer.sender} wants to send ${transfer.fileName}`
        });
      }
    });

    const unlistenProgress = listen<TransferProgressPayload>('transfer_progress', (event) => {
      appState.updateTransfer(event.payload.id, {
        progress: (event.payload.bytesTransferred / event.payload.totalBytes) * 100,
        status: 'streaming',
        speedBps: event.payload.speedBps
      });
    });

    const unlistenCompleted = listen<string>('transfer_completed', (event) => {
      const t = appState.transfers.find(x => x.id === event.payload);
      if (t) {
        appState.updateTransfer(event.payload, {
          progress: 100,
          status: 'completed'
        });
        appState.addToast(`Transfer completed: ${t.fileName}`, 'success');
        if (notificationsEnabled) {
          sendNotification({
            title: 'DropBridge — Transfer Complete',
            body: `${t.fileName} was successfully transferred.`
          });
        }
      }
    });

    const unlistenDeclined = listen<string>('transfer_declined', (event) => {
      appState.updateTransfer(event.payload, { status: 'declined' });
      appState.addToast(`Transfer declined`, 'error');
    });

    const unlistenCancelled = listen<string>('transfer_cancelled', (event) => {
      appState.updateTransfer(event.payload, { status: 'cancelled' });
      appState.addToast(`Transfer cancelled`, 'error');
    });

    const unlistenFailed = listen<{id: string, reason: string}>('transfer_failed', (event) => {
      appState.updateTransfer(event.payload.id, { status: 'failed' });
      appState.addToast(`Transfer failed: ${event.payload.reason}`, 'error');
    });

    const unlistenTimeout = listen<string>('transfer_timeout', (event) => {
      if (appState.pendingTransfer?.id === event.payload) {
        appState.setPending(null);
        appState.addToast(`Transfer request timed out`, 'info');
      }
    });

    return () => {
      unlistenPeer.then(fn => fn());
      unlistenPeerRemoved.then(fn => fn());
      unlistenRequest.then(fn => fn());
      unlistenProgress.then(fn => fn());
      unlistenCompleted.then(fn => fn());
      unlistenDeclined.then(fn => fn());
      unlistenCancelled.then(fn => fn());
      unlistenFailed.then(fn => fn());
      unlistenTimeout.then(fn => fn());
    };
  });

  async function handleResponse(accept: boolean) {
    if (appState.pendingTransfer) {
      const t = { ...appState.pendingTransfer };
      if (accept) {
        t.status = 'streaming';
        appState.addTransfer(t);
      }
      await invoke('respond_to_transfer', { id: t.id, accept });
      appState.setPending(null);
    }
  }
</script>

<div class="flex h-screen bg-background text-text-primary font-sans overflow-hidden">
  <!-- Sidebar -->
  <aside class="w-[240px] bg-surface border-r border-border flex flex-col">
    <div class="p-6 flex items-center gap-2">
      <div class="w-8 h-8 bg-brand rounded-[6px] flex items-center justify-center">
        <Send size={18} class="text-surface" />
      </div>
      <span class="text-xl font-medium tracking-tight">DropBridge</span>
    </div>

    <nav class="flex-1 px-3 py-4 space-y-1">
      {#each navItems as item}
        <a
          href={item.path}
          class="flex items-center gap-3 px-3 py-2 rounded-[6px] transition-colors duration-200 group
            {$page.url.pathname === item.path 
              ? 'bg-brand/10 text-brand border-l-2 border-brand rounded-[6px]' 
              : 'text-text-secondary hover:text-text-primary hover:bg-border-hover'}"
        >
          <item.icon size={18} />
          <span class="text-sm font-medium">{item.name}</span>
        </a>
      {/each}
    </nav>

    <div class="p-4 border-t border-border flex items-center justify-between">
      <span class="text-[10px] font-mono text-text-muted uppercase tracking-widest">Version {version}</span>
      <div class="w-1.5 h-1.5 bg-brand rounded-full animate-pulse"></div>
    </div>
  </aside>

  <!-- Main Content -->
  <main class="flex-1 overflow-y-auto relative">
    {@render children()}

    <!-- Active Transfers Tray -->
    {#if appState.transfers.some(t => t.status === 'streaming')}
      <div class="fixed bottom-0 left-[240px] right-0 bg-surface border-t border-border px-6 py-3 flex items-center justify-between z-40 shadow-lg">
        <div class="flex items-center gap-3">
          <div class="w-2 h-2 bg-brand rounded-full animate-pulse"></div>
          <span class="text-sm font-medium">
            {appState.transfers.filter(t => t.status === 'streaming').length} transfer(s) active
          </span>
        </div>
        <span class="text-xs font-mono text-brand font-medium tracking-wide">
          {formatSpeed(appState.transfers.filter(t => t.status === 'streaming').reduce((acc, t) => acc + (t.speedBps ?? 0), 0))}
        </span>
      </div>
    {/if}

    <!-- Toasts -->
    <div class="fixed bottom-16 right-6 z-[60] flex flex-col gap-3">
      {#each appState.toasts as toast}
        <div class="px-4 py-3 rounded-[6px] border bg-surface shadow-2xl animate-in slide-in-from-right-8 duration-300 flex items-center gap-3
          {toast.type === 'success' ? 'border-brand/40 text-brand' : toast.type === 'error' ? 'border-danger/40 text-danger' : 'border-border-card text-text-primary'}">
          {#if toast.type === 'success'}<Check size={16} />{/if}
          {#if toast.type === 'error'}<X size={16} />{/if}
          <span class="text-sm font-medium">{toast.message}</span>
        </div>
      {/each}
    </div>

    <!-- Consent Modal -->
    {#if appState.pendingTransfer}
      <div class="fixed inset-0 bg-background/80 backdrop-blur-sm flex items-center justify-center z-50 p-4">
        <div class="bg-surface border border-border-card rounded-[32px] w-full max-w-md overflow-hidden shadow-2xl animate-in zoom-in-95 duration-200">
          <div class="p-8">
            <div class="w-16 h-16 bg-brand-translucent rounded-2xl flex items-center justify-center mb-6 mx-auto">
              <File size={32} class="text-brand" />
            </div>
            
            <h2 class="text-2xl font-medium leading-none text-center mb-2">Transfer Request</h2>
            <p class="text-text-secondary text-center mb-8">
              <span class="text-text-primary font-medium">{appState.pendingTransfer.sender}</span> wants to send you a file.
            </p>

            <div class="bg-background border border-border-card rounded-2xl p-4 mb-8">
              <p class="text-sm font-medium text-text-primary truncate">{appState.pendingTransfer.fileName}</p>
              <p class="text-xs font-mono text-text-muted uppercase tracking-tighter">
                {formatSize(appState.pendingTransfer.fileSize)}
              </p>
            </div>

            <div class="flex gap-4">
              <button 
                onclick={() => handleResponse(false)}
                class="flex-1 py-3 bg-surface border border-border-card rounded-full text-sm font-medium text-text-secondary hover:text-text-primary hover:border-text-secondary transition-all active:scale-[0.98]"
              >
                Decline
              </button>
              <button 
                onclick={() => handleResponse(true)}
                class="flex-1 py-3 bg-brand text-surface rounded-full text-sm font-medium transition-all hover:bg-brand-hover hover:scale-[1.02] active:scale-[0.98] shadow-lg shadow-brand/20"
              >
                Accept
              </button>
            </div>
          </div>
        </div>
      </div>
    {/if}
  </main>
</div>
