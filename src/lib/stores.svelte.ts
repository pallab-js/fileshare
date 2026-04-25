import { type Peer, type Transfer, type Toast } from './types';

class TransferStore {
  peers = $state.raw<Peer[]>([]);
  transfers = $state.raw<Transfer[]>([]);
  pendingTransfer = $state<Transfer | null>(null);
  toasts = $state<Toast[]>([]);

  addToast(message: string, type: 'success' | 'error' | 'info' = 'info') {
    const id = crypto.randomUUID();
    this.toasts = [...this.toasts, { id, message, type }];
    setTimeout(() => {
      this.toasts = this.toasts.filter(x => x.id !== id);
    }, 3000);
  }

  setPeers(list: Peer[]) {
    this.peers = list;
  }

  updatePeer(peer: Peer) {
    const list = [...this.peers];
    const index = list.findIndex(p => p.id === peer.id);
    if (index > -1) {
      list[index] = peer;
      this.peers = list;
    } else {
      this.peers = [...list, peer];
    }
  }

  removePeer(id: string) {
    this.peers = this.peers.filter(p => p.id !== id);
  }

  addTransfer(t: Transfer) {
    this.transfers = [t, ...this.transfers];
  }

  updateTransfer(id: string, update: Partial<Transfer>) {
    this.transfers = this.transfers.map(t => 
      t.id === id ? { ...t, ...update } : t
    );
  }

  setPending(t: Transfer | null) {
    this.pendingTransfer = t;
  }
}

export const appState = new TransferStore();
