<script lang="ts">
  import { User, Bell, Shield, FolderOpen, Save, Check } from 'lucide-svelte';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { addToast } from '$lib/stores';

  let settings = $state({
    displayName: '',
    autoAccept: false,
    saveDirectory: 'Downloads/DropBridge',
    notifications: true
  });

  let isLoading = $state(true);
  let isSaving = $state(false);

  onMount(async () => {
    try {
      const saved = await invoke<Record<string, string>>('get_settings');
      if (saved.displayName) settings.displayName = saved.displayName;
      if (saved.autoAccept) settings.autoAccept = saved.autoAccept === 'true';
      if (saved.saveDirectory) settings.saveDirectory = saved.saveDirectory;
      if (saved.notifications) settings.notifications = saved.notifications === 'true';
    } catch (e) {
      console.error("Failed to load settings", e);
    } finally {
      isLoading = false;
    }
  });

  async function handleSave() {
    isSaving = true;
    try {
      const toSave = {
        displayName: settings.displayName,
        autoAccept: settings.autoAccept.toString(),
        saveDirectory: settings.saveDirectory,
        notifications: settings.notifications.toString()
      };
      await invoke('save_settings', { settings: toSave });
      addToast('Settings saved successfully', 'success');
    } catch (e: any) {
      addToast(e.toString(), 'error');
    } finally {
      isSaving = false;
    }
  }
</script>

<div class="p-8 max-w-3xl mx-auto">
  <div class="flex items-center justify-between mb-8">
    <div>
      <h1 class="text-3xl font-medium tracking-tight mb-2 leading-none">Settings</h1>
      <p class="text-text-secondary">Manage your device and transfer preferences.</p>
    </div>
    <button 
      onclick={handleSave}
      disabled={isSaving || isLoading}
      class="flex items-center gap-2 px-6 py-2 bg-brand text-surface rounded-[6px] text-sm font-medium transition-all hover:bg-brand-hover active:scale-[0.98] disabled:opacity-50"
    >
      {#if isSaving}
        <div class="w-4 h-4 border-2 border-surface/30 border-t-surface rounded-full animate-spin"></div>
      {:else}
        <Save size={18} />
      {/if}
      Save Changes
    </button>
  </div>

  {#if isLoading}
    <div class="py-20 flex justify-center">
      <div class="w-8 h-8 border-4 border-brand-translucent border-t-brand rounded-full animate-spin"></div>
    </div>
  {:else}
    <div class="space-y-6">
      <!-- Profile -->
      <section class="bg-surface border border-border-card rounded-2xl overflow-hidden">
        <div class="p-6 border-b border-border-card bg-background/30 flex items-center gap-4">
          <div class="w-12 h-12 bg-brand rounded-full flex items-center justify-center text-surface font-medium text-lg">
            {settings.displayName ? settings.displayName.charAt(0).toUpperCase() : 'U'}
          </div>
          <div>
            <h2 class="text-lg font-medium leading-none">Device Identity</h2>
            <p class="text-xs text-text-muted">How your device appears to others.</p>
          </div>
        </div>
        <div class="p-6">
          <label class="block text-xs font-mono uppercase tracking-widest text-text-muted mb-2" for="name">Display Name</label>
          <input 
            type="text" 
            id="name"
            bind:value={settings.displayName}
            placeholder="e.g. My MacBook Pro"
            class="w-full bg-background border border-border-card rounded-[6px] px-4 py-2 text-sm focus:outline-none focus:border-brand/50 transition-colors"
          />
        </div>
      </section>

      <!-- Transfers -->
      <section class="bg-surface border border-border-card rounded-2xl overflow-hidden">
        <div class="p-6 border-b border-border-card bg-background/30 flex items-center gap-3">
          <FolderOpen size={20} class="text-brand" />
          <h2 class="text-lg font-medium leading-none">Transfer Preferences</h2>
        </div>
        <div class="p-6 space-y-6">
          <div class="flex items-center justify-between">
            <div>
              <p class="text-sm font-medium text-text-primary">Auto-accept Transfers</p>
              <p class="text-xs text-text-muted">Receive files automatically without confirmation.</p>
            </div>
            <button 
              onclick={() => settings.autoAccept = !settings.autoAccept}
              aria-label="Toggle auto-accept transfers"
              class="w-12 h-6 rounded-full transition-colors relative {settings.autoAccept ? 'bg-brand' : 'bg-border-card'}"
            >
              <div class="absolute top-1 left-1 w-4 h-4 bg-surface rounded-full transition-transform {settings.autoAccept ? 'translate-x-6' : ''}"></div>
            </button>
          </div>

          <div class="flex items-center justify-between">
            <div>
              <p class="text-sm font-medium text-text-primary">Desktop Notifications</p>
              <p class="text-xs text-text-muted">Get alerted when a transfer starts or ends.</p>
            </div>
            <button 
              onclick={() => settings.notifications = !settings.notifications}
              aria-label="Toggle desktop notifications"
              class="w-12 h-6 rounded-full transition-colors relative {settings.notifications ? 'bg-brand' : 'bg-border-card'}"
            >
              <div class="absolute top-1 left-1 w-4 h-4 bg-surface rounded-full transition-transform {settings.notifications ? 'translate-x-6' : ''}"></div>
            </button>
          </div>

          <div>
            <label class="block text-xs font-mono uppercase tracking-widest text-text-muted mb-2" for="save-location">Default Save Location</label>
            <div class="flex gap-2">
              <input 
                type="text" 
                readonly
                id="save-location"
                value={settings.saveDirectory}
                class="flex-1 bg-background border border-border-card rounded-[6px] px-4 py-2 text-sm text-text-muted cursor-not-allowed"
              />
              <button class="px-4 py-2 bg-background border border-border-card rounded-[6px] text-xs font-medium text-text-secondary hover:text-text-primary transition-colors">
                Change
              </button>
            </div>
          </div>
        </div>
      </section>

      <!-- Admin -->
      <section class="bg-surface border border-border-card rounded-2xl overflow-hidden opacity-60">
        <div class="p-6 border-b border-border-card bg-background/30 flex items-center gap-3">
          <Shield size={20} class="text-brand" />
          <h2 class="text-lg font-medium leading-none">Advanced (Admin Only)</h2>
        </div>
        <div class="p-6">
          <p class="text-sm text-text-muted italic">Administrator features are locked to this local session.</p>
        </div>
      </section>
    </div>
  {/if}
</div>
