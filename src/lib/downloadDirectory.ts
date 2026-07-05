import { downloadDir } from '@tauri-apps/api/path';
import { ref } from 'vue';

export const defaultDownloadDirectory = ref('');

export async function initializeDownloadDirectory(): Promise<void> {
    defaultDownloadDirectory.value = await downloadDir();
}

export function displayDownloadDirectory(directory: string): string {
    if (directory !== 'default') return directory;
    return defaultDownloadDirectory.value || 'ダウンロードフォルダー';
}
