import { Store } from '@tauri-apps/plugin-store';
import { ref, watch } from 'vue';

const STORE_PATH = 'settings.json';
const SETTINGS_KEY = 'settings';
const MIGRATION_KEY = 'localStorageMigrated';
const LEGACY_STORAGE_KEY = 'otosu-settings';

const defaults = {
    defaultCookieProfilePath: 'none',
    defaultFormat: 'mp4' as 'mp4' | 'mkv' | 'mp3' | 'flac' | 'wav',
    defaultQuality: 'best',
    defaultEmbedSubtitles: false,
    defaultEmbedThumbnail: false,
    defaultEmbedMetadata: false,
    defaultPlaylistBehavior: 'no' as 'yes' | 'no',
    defaultWriteAutoSubs: false,
    defaultSubLangs: 'ja' as 'ja' | 'en' | 'all',
    defaultDownloadDir: 'default',
};

export type GlobalSettings = typeof defaults;

export const globalSettings = ref<GlobalSettings>({ ...defaults });

let settingsStore: Store | null = null;

function sanitizeSettings(value: unknown): GlobalSettings {
    if (!value || typeof value !== 'object') return { ...defaults };

    const candidate = value as Partial<GlobalSettings>;
    const settings = { ...defaults };

    for (const key of Object.keys(defaults) as Array<keyof GlobalSettings>) {
        if (typeof candidate[key] === typeof defaults[key]) {
            Object.assign(settings, { [key]: candidate[key] });
        }
    }

    if (!['mp4', 'mkv', 'mp3', 'flac', 'wav'].includes(settings.defaultFormat)) {
        settings.defaultFormat = defaults.defaultFormat;
    }
    if (!['yes', 'no'].includes(settings.defaultPlaylistBehavior)) {
        settings.defaultPlaylistBehavior = defaults.defaultPlaylistBehavior;
    }
    if (!['ja', 'en', 'all'].includes(settings.defaultSubLangs)) {
        settings.defaultSubLangs = defaults.defaultSubLangs;
    }

    return settings;
}

function readLegacySettings(): unknown {
    const serialized = localStorage.getItem(LEGACY_STORAGE_KEY);
    if (!serialized) return undefined;

    try {
        return JSON.parse(serialized);
    } catch {
        return undefined;
    }
}

export async function initializeGlobalSettings(): Promise<void> {
    settingsStore = await Store.load(STORE_PATH, {
        defaults: { [SETTINGS_KEY]: defaults },
        autoSave: 200,
    });

    const migrationComplete = await settingsStore.get<boolean>(MIGRATION_KEY);
    let storedSettings = await settingsStore.get<GlobalSettings>(SETTINGS_KEY);

    if (!migrationComplete) {
        const legacySettings = readLegacySettings();
        if (legacySettings) {
            storedSettings = sanitizeSettings(legacySettings);
            await settingsStore.set(SETTINGS_KEY, storedSettings);
        }
        await settingsStore.set(MIGRATION_KEY, true);
        await settingsStore.save();
        localStorage.removeItem(LEGACY_STORAGE_KEY);
    }

    globalSettings.value = sanitizeSettings(storedSettings);

    watch(
        globalSettings,
        settings => {
            void settingsStore?.set(SETTINGS_KEY, settings);
        },
        { deep: true },
    );
}

export function resetGlobalSettings(): void {
    globalSettings.value = { ...defaults };
}
