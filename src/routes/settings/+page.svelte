<script lang="ts">
  import { User, Monitor, Bell, ShieldCheck } from 'lucide-svelte';

  let settings = $state({
    displayName: 'Pallab PC',
    autoAccept: false,
    saveDirectory: '~/Downloads/DropBridge',
    notifications: true,
  });
</script>

<div class="p-8 max-w-3xl mx-auto">
  <h1 class="text-3xl font-medium tracking-tight mb-8 leading-none">Settings</h1>

  <div class="space-y-6">
    <!-- Profile -->
    <section class="bg-surface border border-border-card rounded-2xl p-6">
      <div class="flex items-center gap-4 mb-6">
        <div class="w-16 h-16 bg-background border border-border-card rounded-2xl flex items-center justify-center text-2xl font-medium text-brand">
          P
        </div>
        <div>
          <h2 class="text-lg font-medium leading-none">Device Profile</h2>
          <p class="text-sm text-text-muted mt-2">How others see you on the network.</p>
        </div>
      </div>
      
      <div class="space-y-4">
        <div>
          <label class="block text-xs font-mono uppercase tracking-widest text-text-muted mb-2" for="name">Display Name</label>
          <input 
            type="text" 
            bind:value={settings.displayName}
            class="w-full bg-background border border-border-card rounded-[6px] px-4 py-2 text-sm focus:outline-none focus:border-brand/50 transition-colors"
          />
        </div>
      </div>
    </section>

    <!-- Transfer Settings -->
    <section class="bg-surface border border-border-card rounded-2xl p-6">
      <h2 class="text-lg font-medium mb-4 leading-none">Transfer Preferences</h2>
      
      <div class="space-y-4">
        <div class="flex items-center justify-between py-2">
          <div class="flex items-center gap-3">
            <ShieldCheck size={18} class="text-text-muted" />
            <div>
              <p class="text-sm font-medium">Auto-approve transfers</p>
              <p class="text-xs text-text-muted">Receive files without manual confirmation.</p>
            </div>
          </div>
          <button 
            onclick={() => settings.autoAccept = !settings.autoAccept}
            aria-label="Toggle auto-approve transfers"
            class="w-10 h-5 rounded-full transition-colors relative {settings.autoAccept ? 'bg-brand' : 'bg-background border border-border-card'}"
          >
            <div class="absolute top-1 left-1 w-3 h-3 rounded-full bg-white transition-transform {settings.autoAccept ? 'translate-x-5' : ''}"></div>
          </button>
        </div>

        <div class="flex items-center justify-between py-2 border-t border-border-card/50">
          <div class="flex items-center gap-3">
            <Bell size={18} class="text-text-muted" />
            <div>
              <p class="text-sm font-medium">Desktop Notifications</p>
              <p class="text-xs text-text-muted">Show alerts for new transfer requests.</p>
            </div>
          </div>
          <button 
            onclick={() => settings.notifications = !settings.notifications}
            aria-label="Toggle desktop notifications"
            class="w-10 h-5 rounded-full transition-colors relative {settings.notifications ? 'bg-brand' : 'bg-background border border-border-card'}"
          >
            <div class="absolute top-1 left-1 w-3 h-3 rounded-full bg-white transition-transform {settings.notifications ? 'translate-x-5' : ''}"></div>
          </button>
        </div>
      </div>
    </section>

    <div class="flex justify-end pt-4">
      <button class="px-6 py-2 bg-brand-translucent text-brand font-medium rounded-[6px] border border-brand/20 hover:bg-brand/20 transition-all">
        Save Changes
      </button>
    </div>
  </div>
</div>
