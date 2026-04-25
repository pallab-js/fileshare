import { writable } from 'svelte/store';

export interface Peer {
  id: string;
  name: string;
  ip: string;
  port: number;
  last_seen: number;
}

export interface Transfer {
  id: string;
  fileName: string;
  fileSize: number;
  sender: string;
  progress: number;
  status: 'pending' | 'streaming' | 'completed' | 'cancelled' | 'declined';
}

export const peers = writable<Peer[]>([]);
export const transfers = writable<Transfer[]>([]);
export const pendingTransfer = writable<Transfer | null>(null);
