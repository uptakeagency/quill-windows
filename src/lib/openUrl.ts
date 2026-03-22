import { openUrl as tauriOpenUrl } from '@tauri-apps/plugin-opener';

export function openUrl(url: string) {
  tauriOpenUrl(url).catch(() => {
    // Fallback: try window.open
    window.open(url, '_blank');
  });
}
