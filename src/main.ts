import "@fontsource-variable/inter";
import "@fontsource-variable/noto-sans-jp";
import { createApp } from "vue";
import "@/style.css";
import App from "./App.vue";
import { initializeGlobalSettings } from "@/lib/globalSettings";
import { initializeDownloadDirectory } from "@/lib/downloadDirectory";
import { getCurrent, onOpenUrl } from '@tauri-apps/plugin-deep-link';

function shareYouTubeUrls(urls: string[]) {
    for (const value of urls) {
        try {
            const deepLink = new URL(value);
            const sharedUrl = deepLink.protocol === 'otosu:' && deepLink.hostname === 'download'
                ? deepLink.searchParams.get('url')
                : null;
            if (!sharedUrl) continue;
            const videoUrl = new URL(sharedUrl);
            if (!['youtube.com', 'www.youtube.com', 'm.youtube.com', 'youtu.be', 'music.youtube.com'].includes(videoUrl.hostname)) continue;
            window.dispatchEvent(new CustomEvent('otosu-share-url', { detail: videoUrl.toString() }));
        } catch {
            // Ignore malformed or unsupported deep links.
        }
    }
}

async function bootstrap() {
    const [settingsResult, directoryResult] = await Promise.allSettled([
        initializeGlobalSettings(),
        initializeDownloadDirectory(),
    ]);

    if (settingsResult.status === 'rejected') {
        console.error("設定の読み込みに失敗しました:", settingsResult.reason);
    }
    if (directoryResult.status === 'rejected') {
        console.error("デフォルト保存先の取得に失敗しました:", directoryResult.reason);
    }

    createApp(App).mount("#app");

    const initialUrls = await getCurrent();
    if (initialUrls) shareYouTubeUrls(initialUrls);
    await onOpenUrl(shareYouTubeUrls);
}

void bootstrap();
