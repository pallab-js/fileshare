export interface Peer {
  id: string;
  name: string;
  ip: string;
  port: number;
  last_seen: number;
  device_type: string;
}

export interface Transfer {
  id: string;
  fileName: string;
  fileSize: number;
  sender: string;
  progress: number;
  status: 'pending' | 'streaming' | 'completed' | 'cancelled' | 'declined' | 'failed';
  speedBps?: number;
  timestamp?: number;
}

export interface TransferRequestPayload {
  id: string;
  file_name: string;
  file_size: number;
  sender: string;
}

export interface TransferProgressPayload {
  id: string;
  bytesTransferred: number;
  totalBytes: number;
  speedBps: number;
}

export interface Toast {
  id: string;
  message: string;
  type: 'success' | 'error' | 'info';
}
