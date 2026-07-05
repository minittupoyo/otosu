import { ref } from 'vue';
import { listen } from '@tauri-apps/api/event';

export interface PlaylistEntry {
    title?: string;
    duration?: number;
    thumbnail?: string;
    playlist_index?: number;
}

export interface DownloadTask {
    id: string;
    title: string;
    duration: string;
    thumbnail?: string;
    url: string;
    format: 'mp4' | 'mkv' | 'mp3' | 'flac' | 'wav';
    quality: string;
    progress: number;
    status: 'downloading' | 'done' | 'failed';
    speed: string;
    eta: string;
    error?: string;
    createdAt: string;
    embedSubtitles: boolean;
    embedThumbnail: boolean;
    embedMetadata: boolean;
    playlistBehavior: 'yes' | 'no';
    writeAutoSubs: boolean;
    subLangs: 'ja' | 'en' | 'all';
    cookieProfilePath: string;
    downloadDir: string;
    isPlaylist: boolean;
    playlistEntries: PlaylistEntry[];
    selectedPlaylistItems: number[];
}

export const tasks = ref<DownloadTask[]>([]);

export function addTask(task: DownloadTask) {
    tasks.value.unshift(task);
}

export function restartTask(task: DownloadTask) {
    task.progress = 0;
    task.status = 'downloading';
    task.speed = '';
    task.eta = '';
    task.error = undefined;
    task.createdAt = new Date().toLocaleTimeString();
}

interface ProgressPayload {
    task_id: string;
    progress: number;
    status: 'downloading' | 'done' | 'failed';
    speed: string;
    eta: string;
    error: string | null;
}

let isListening = false;

export function initTaskListener() {
    if (isListening) return;
    isListening = true;

    listen<ProgressPayload>('download-progress', (event) => {
        const { task_id, progress, status, speed, eta, error } = event.payload;
        const task = tasks.value.find(t => t.id === task_id);
        if (task) {
            task.progress = progress;
            task.status = status;
            task.speed = speed;
            task.eta = eta;
            if (error) {
                task.error = error;
            }
        }
    }).catch((err) => {
        console.warn("Tauri event listener failed to initialize:", err);
        isListening = false;
    });
}
