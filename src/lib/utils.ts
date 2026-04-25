export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

export function formatSpeed(bps: number): string {
  return formatSize(bps) + '/s';
}

export function formatETA(bytesRemaining: number, bps: number): string {
  if (!bps || bps === 0) return '--';
  const seconds = Math.round(bytesRemaining / bps);
  if (seconds < 60) return `~${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  return `~${minutes}m ${remainingSeconds}s`;
}
