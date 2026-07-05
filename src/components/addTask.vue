<script setup lang="ts">
import { Dialog, DialogContent, DialogClose, DialogDescription, DialogHeader, DialogFooter, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { computed, nextTick, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Item, ItemActions, ItemContent, ItemTitle, ItemDescription, ItemMedia, ItemGroup } from "@/components/ui/item";
import { IconAdjustments, IconChevronDown, IconChevronUp, IconVideo } from "@tabler/icons-vue";
import { Select, SelectContent, SelectGroup, SelectItem, SelectLabel, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Checkbox } from "@/components/ui/checkbox";
import { globalSettings } from "@/lib/globalSettings";
import { displayDownloadDirectory } from "@/lib/downloadDirectory";
import { open } from '@tauri-apps/plugin-dialog';
import { addTask, restartTask, tasks, type DownloadTask, type PlaylistEntry } from "@/lib/taskState";

const isOpen = ref(false);
const step = ref(1);

const url = ref('');
const videoInfo = ref<{ title: string; duration: string; thumbnail?: string } | null>(null);

interface ProfileInfo {
    name: string;
    path: string;
    is_default: boolean;
}

const downloadFormat = ref<'mp4' | 'mkv' | 'mp3' | 'flac' | 'wav'>('mp4');
const downloadQuality = ref<string>('best');
const embedSubtitles = ref(false);
const embedThumbnail = ref(false);
const embedMetadata = ref(false);
const playlistBehavior = ref<'yes' | 'no'>('no');
const isPlaylist = ref(false);
const playlistEntries = ref<PlaylistEntry[]>([]);
const selectedPlaylistItems = ref<number[]>([]);
const showPlaylistSelection = ref(false);
const showAdvancedSettings = ref(false);
const fetchError = ref('');
const retryingTask = ref<DownloadTask | null>(null);
const isApplyingSettings = ref(false);
const writeAutoSubs = ref(false);
const subLangs = ref<'ja' | 'en' | 'all'>('ja');
const cookieProfilePath = ref<string>('none');
const downloadDir = ref<string>('default');
const hasPlaylistSelection = computed(() => playlistBehavior.value !== 'yes' || selectedPlaylistItems.value.length > 0);

const isPlaylistEntrySelected = (index: number) => {
    return playlistBehavior.value === 'yes' && selectedPlaylistItems.value.includes(index + 1);
};

const togglePlaylistItem = (index: number, selected: boolean) => {
    const position = index + 1;
    selectedPlaylistItems.value = selected
        ? [...new Set([...selectedPlaylistItems.value, position])].sort((a, b) => a - b)
        : selectedPlaylistItems.value.filter(item => item !== position);
};

const selectAllPlaylistItems = () => {
    selectedPlaylistItems.value = playlistEntries.value.map((_, index) => index + 1);
};

const clearPlaylistItems = () => {
    selectedPlaylistItems.value = [];
};

const formatDuration = (duration?: number) => {
    if (!duration) return '長さ不明';
    const hours = Math.floor(duration / 3600);
    const minutes = Math.floor((duration % 3600) / 60);
    const seconds = Math.floor(duration % 60).toString().padStart(2, '0');
    return hours > 0 ? `${hours}:${minutes.toString().padStart(2, '0')}:${seconds}` : `${minutes}:${seconds}`;
};

const selectDownloadDir = async () => {
    try {
        const selected = await open({
            directory: true,
            multiple: false,
            defaultPath: downloadDir.value !== 'default' ? downloadDir.value : undefined
        });
        if (selected && typeof selected === 'string') {
            downloadDir.value = selected;
        }
    } catch (err) {
        console.error(err);
    }
};

const resetDownloadDir = () => {
    downloadDir.value = 'default';
};

const firefoxProfiles = ref<ProfileInfo[]>([]);
const floorpProfiles = ref<ProfileInfo[]>([]);
const zenProfiles = ref<ProfileInfo[]>([]);

const loadAllProfiles = async () => {
    try {
        const [ff, fl, zn] = await Promise.all([
            invoke<ProfileInfo[]>('get_browser_profiles', { browser: 'firefox' }).catch(() => []),
            invoke<ProfileInfo[]>('get_browser_profiles', { browser: 'floorp' }).catch(() => []),
            invoke<ProfileInfo[]>('get_browser_profiles', { browser: 'zen' }).catch(() => [])
        ]);
        firefoxProfiles.value = ff;
        floorpProfiles.value = fl;
        zenProfiles.value = zn;

        if (cookieProfilePath.value !== 'none') {
            const exists = [...ff, ...fl, ...zn].some(p => p.path === cookieProfilePath.value);
            if (!exists) {
                cookieProfilePath.value = 'none';
            }
        }
    } catch (err) {
        console.error(err);
    }
};

watch(downloadFormat, () => {
    if (!isApplyingSettings.value) downloadQuality.value = 'best';
});

watch(isOpen, (newVal) => {
    if (newVal) {
        isApplyingSettings.value = true;
        const task = retryingTask.value;
        downloadFormat.value = task?.format || globalSettings.value.defaultFormat || 'mp4';
        downloadQuality.value = task?.quality || globalSettings.value.defaultQuality || 'best';
        embedSubtitles.value = task?.embedSubtitles ?? globalSettings.value.defaultEmbedSubtitles ?? false;
        embedThumbnail.value = task?.embedThumbnail ?? globalSettings.value.defaultEmbedThumbnail ?? false;
        embedMetadata.value = task?.embedMetadata ?? globalSettings.value.defaultEmbedMetadata ?? false;
        playlistBehavior.value = task?.playlistBehavior || globalSettings.value.defaultPlaylistBehavior || 'no';
        writeAutoSubs.value = task?.writeAutoSubs ?? globalSettings.value.defaultWriteAutoSubs ?? false;
        subLangs.value = task?.subLangs || globalSettings.value.defaultSubLangs || 'ja';
        cookieProfilePath.value = task?.cookieProfilePath || globalSettings.value.defaultCookieProfilePath || 'none';
        downloadDir.value = task?.downloadDir || globalSettings.value.defaultDownloadDir || 'default';
        if (task) {
            url.value = task.url;
            videoInfo.value = { title: task.title, duration: task.duration, thumbnail: task.thumbnail };
            isPlaylist.value = task.isPlaylist;
            playlistEntries.value = [...task.playlistEntries];
            selectedPlaylistItems.value = [...task.selectedPlaylistItems];
            step.value = 3;
            showAdvancedSettings.value = true;
        }
        void nextTick(() => { isApplyingSettings.value = false; });
        loadAllProfiles();
    } else {
        step.value = 1
        url.value = ''
        videoInfo.value = null
        isPlaylist.value = false
        playlistEntries.value = []
        selectedPlaylistItems.value = []
        showPlaylistSelection.value = false
        showAdvancedSettings.value = false
        fetchError.value = ''
        firefoxProfiles.value = []
        floorpProfiles.value = []
        zenProfiles.value = []
        cookieProfilePath.value = 'none'
        downloadDir.value = 'default'
        retryingTask.value = null
    }
})

interface VideoInfoResponse {
    title: string;
    duration?: number;
    thumbnail?: string;
    _type?: string;
    entries?: PlaylistEntry[];
}

const handleFetchInfo = async () => {
    if (!url.value) return

    fetchError.value = ''
    step.value = 2

    try {
        const res = await invoke<VideoInfoResponse>('fetch_video_info', { url: url.value })
        
        if (res._type === 'playlist') {
            const count = res.entries?.length || 0;
            const totalDuration = res.entries?.reduce((total, entry) => total + (entry.duration || 0), 0) || 0;
            const durationLabel = totalDuration > 0
                ? `・合計 ${Math.floor(totalDuration / 3600)}時間${Math.floor((totalDuration % 3600) / 60)}分`
                : '';
            videoInfo.value = {
                title: res.title,
                duration: `${count} 本の動画${durationLabel}`,
                thumbnail: res.thumbnail || res.entries?.[0]?.thumbnail
            };
            isPlaylist.value = true;
            playlistEntries.value = res.entries || [];
            selectAllPlaylistItems();
            playlistBehavior.value = 'yes';
        } else {
            const minutes = Math.floor((res.duration || 0) / 60);
            const seconds = Math.floor((res.duration || 0) % 60).toString().padStart(2, '0');
            videoInfo.value = {
                title: res.title,
                duration: `${minutes}:${seconds}`,
                thumbnail: res.thumbnail
            };
            isPlaylist.value = false;
            playlistEntries.value = [];
            selectedPlaylistItems.value = [];
            playlistBehavior.value = 'no';
        }

        console.log(videoInfo.value.title)

        step.value = 3
    } catch (err) {
        console.error(err)
        fetchError.value = String(err)
        step.value = 1
    }
}

const openForRetry = (task: DownloadTask) => {
    retryingTask.value = task;
    isOpen.value = true;
};

const openWithUrl = async (sharedUrl: string) => {
    retryingTask.value = null;
    isOpen.value = true;
    await nextTick();
    url.value = sharedUrl;
    await handleFetchInfo();
};

defineExpose({ openForRetry, openWithUrl });

const handleAddTask = async () => {
    if (!url.value || !videoInfo.value || !hasPlaylistSelection.value) return;

    const taskId = retryingTask.value?.id || Date.now().toString();
    const taskData: DownloadTask = {
        id: taskId,
        title: videoInfo.value.title,
        duration: videoInfo.value.duration,
        thumbnail: videoInfo.value.thumbnail,
        url: url.value,
        format: downloadFormat.value,
        quality: downloadQuality.value,
        progress: 0,
        status: 'downloading' as const,
        speed: '',
        eta: '',
        createdAt: retryingTask.value?.createdAt || new Date().toLocaleTimeString(),
        embedSubtitles: embedSubtitles.value,
        embedThumbnail: embedThumbnail.value,
        embedMetadata: embedMetadata.value,
        playlistBehavior: playlistBehavior.value,
        writeAutoSubs: writeAutoSubs.value,
        subLangs: subLangs.value,
        cookieProfilePath: cookieProfilePath.value,
        downloadDir: downloadDir.value,
        isPlaylist: isPlaylist.value,
        playlistEntries: [...playlistEntries.value],
        selectedPlaylistItems: [...selectedPlaylistItems.value],
    };

    const existingTask = retryingTask.value;
    if (existingTask) {
        Object.assign(existingTask, taskData);
        restartTask(existingTask);
    } else {
        addTask(taskData);
    }

    const commandArgs = {
        taskId,
        url: taskData.url,
        format: taskData.format,
        quality: taskData.quality,
        embedSubtitles: taskData.embedSubtitles,
        embedThumbnail: taskData.embedThumbnail,
        embedMetadata: taskData.embedMetadata,
        playlistBehavior: taskData.playlistBehavior,
        writeAutoSubs: taskData.writeAutoSubs,
        subLangs: taskData.subLangs,
        cookieProfilePath: taskData.cookieProfilePath,
        downloadDir: taskData.downloadDir,
        playlistItems: taskData.isPlaylist && taskData.playlistBehavior === 'yes'
            ? taskData.selectedPlaylistItems
            : null,
    };
    isOpen.value = false;

    try {
        await invoke('start_download', commandArgs);
    } catch (err) {
        console.error("Failed to start download:", err);
        const task = tasks.value.find(t => t.id === taskId);
        if (task) {
            task.status = 'failed';
            task.error = String(err);
        }
    }
}
</script>



<template>
    <Dialog v-model:open="isOpen">
        <DialogTrigger as-child>
            <Button>ダウンロードを追加</Button>
        </DialogTrigger>
        <DialogContent :class="isPlaylist ? 'sm:max-w-2xl' : undefined">
            <template v-if="step === 1">
                <DialogHeader>
                    <DialogTitle>ダウンロードを追加</DialogTitle>
                    <DialogDescription>動画またはプレイリストのURLを入力してください。</DialogDescription>
                </DialogHeader>
                <div class="grid gap-4 py-4">
                    <Input type="url" placeholder="https://..." v-model="url" aria-label="動画URL" />
                    <p v-if="fetchError" class="text-sm text-destructive">動画情報を取得できませんでした。URLやCookie設定を確認してください。</p>
                </div>
                <DialogFooter>
                    <DialogClose as-child>
                        <Button variant="ghost">キャンセル</Button>
                    </DialogClose>
                    <Button :disabled="!url" type="button" @click="handleFetchInfo">次へ</Button>
                </DialogFooter>
            </template>
            <template v-if="step === 2">
                <DialogHeader>
                    <DialogTitle>動画情報を取得しています</DialogTitle>
                </DialogHeader>
                <div class="flex flex-col items-center gap-4 justify-center py-8">
                    <Spinner class="size-8" />
                    <p class="text-sm">情報を取得しています...</p>
                </div>
            </template>
            <template v-if="step === 3">
                <DialogHeader>
                    <DialogTitle>{{ retryingTask ? '設定を変更して再実行' : 'ダウンロード内容を確認' }}</DialogTitle>
                    <DialogDescription>{{ retryingTask ? '失敗したダウンロードの設定を確認してください。' : '基本設定を確認してダウンロードを開始します。' }}</DialogDescription>
                </DialogHeader>
                <div class="py-4 flex flex-col gap-4 max-h-[50vh] overflow-y-auto pr-2">
                    <Item variant="outline">
                        <ItemMedia :variant="videoInfo?.thumbnail ? 'image' : 'icon'" :style="videoInfo?.thumbnail ? 'width: 96px; height: auto; aspect-ratio: 16/9;' : ''">
                            <img v-if="videoInfo?.thumbnail" :src="videoInfo.thumbnail" alt="Thumbnail" />
                            <IconVideo v-else class="size-4" />
                        </ItemMedia>
                        <ItemContent>
                            <ItemTitle>{{ videoInfo?.title }}</ItemTitle>
                            <ItemDescription>{{ videoInfo?.duration }}</ItemDescription>
                        </ItemContent>
                    </Item>

                    <template v-if="isPlaylist">
                        <div class="flex flex-col gap-2">
                            <div class="flex items-center justify-between gap-2">
                                <div>
                                    <Label>動画の選択</Label>
                                    <p v-if="playlistBehavior === 'yes'" class="text-sm text-muted-foreground">
                                        {{ selectedPlaylistItems.length }} / {{ playlistEntries.length }} 本を選択中
                                    </p>
                                </div>
                                <Button v-if="playlistBehavior === 'yes'" type="button" variant="outline" size="sm" @click="showPlaylistSelection = !showPlaylistSelection">
                                    {{ showPlaylistSelection ? '閉じる' : '選択を変更' }}
                                    <IconChevronUp v-if="showPlaylistSelection" data-icon="inline-end" />
                                    <IconChevronDown v-else data-icon="inline-end" />
                                </Button>
                            </div>
                            <div v-if="showPlaylistSelection && playlistBehavior === 'yes'" class="flex items-center justify-end gap-1">
                                <Button type="button" variant="ghost" size="sm" @click="selectAllPlaylistItems">すべて選択</Button>
                                <Button type="button" variant="ghost" size="sm" @click="clearPlaylistItems">選択解除</Button>
                            </div>
                            <div v-if="showPlaylistSelection" class="max-h-52 overflow-y-auto rounded-md border">
                                <ItemGroup>
                                    <Item
                                        v-for="(entry, index) in playlistEntries"
                                        :key="entry.playlist_index || index"
                                        :variant="isPlaylistEntrySelected(index) ? 'outline' : 'default'"
                                        size="sm"
                                    >
                                        <ItemMedia v-if="entry.thumbnail" variant="image">
                                            <img :src="entry.thumbnail" alt="" />
                                        </ItemMedia>
                                        <ItemContent>
                                            <ItemTitle>{{ entry.playlist_index || index + 1 }}. {{ entry.title || 'タイトル不明' }}</ItemTitle>
                                            <ItemDescription>
                                                {{ formatDuration(entry.duration) }}
                                                <template v-if="isPlaylistEntrySelected(index)">・ダウンロード対象</template>
                                            </ItemDescription>
                                        </ItemContent>
                                        <ItemActions v-if="playlistBehavior === 'yes'">
                                            <Checkbox
                                                :checked="isPlaylistEntrySelected(index)"
                                                :aria-label="`${entry.title || `動画 ${index + 1}`}を選択`"
                                                @update:checked="togglePlaylistItem(index, $event === true)"
                                            />
                                        </ItemActions>
                                    </Item>
                                </ItemGroup>
                            </div>
                        </div>

                        <div class="flex flex-col gap-2">
                            <Label for="detected-playlist-behavior">ダウンロード対象</Label>
                            <Select v-model="playlistBehavior">
                                <SelectTrigger id="detected-playlist-behavior" class="w-full">
                                    <SelectValue placeholder="対象を選択" />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem value="yes">プレイリストをダウンロード</SelectItem>
                                    <SelectItem value="no">URLで指定された動画のみダウンロード</SelectItem>
                                </SelectContent>
                            </Select>
                        </div>

                        <p v-if="playlistBehavior === 'yes' && !hasPlaylistSelection" class="text-sm text-destructive">
                            ダウンロードする動画を1本以上選択してください。
                        </p>
                    </template>

                    <Separator />

                    <div class="grid gap-4">
                        <div class="flex flex-col gap-2">
                            <Label for="download-format">保存形式</Label>
                            <Select v-model="downloadFormat">
                                <SelectTrigger id="download-format" class="w-full">
                                    <SelectValue placeholder="保存形式を選択" />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem value="mp4">動画 (MP4)</SelectItem>
                                    <SelectItem value="mkv">動画 (MKV)</SelectItem>
                                    <SelectItem value="mp3">音声 (MP3)</SelectItem>
                                    <SelectItem value="flac">音声 (FLAC)</SelectItem>
                                    <SelectItem value="wav">音声 (WAV)</SelectItem>
                                </SelectContent>
                            </Select>
                        </div>

                        <div class="flex flex-col gap-2">
                            <Label for="download-quality">品質</Label>
                            <Select v-model="downloadQuality">
                                <SelectTrigger id="download-quality" class="w-full">
                                    <SelectValue placeholder="品質を選択" />
                                </SelectTrigger>
                                <SelectContent v-if="downloadFormat === 'mp4' || downloadFormat === 'mkv'">
                                    <SelectItem value="best">自動（最高画質）</SelectItem>
                                    <SelectItem value="2160p">4K (2160p)</SelectItem>
                                    <SelectItem value="1440p">2K (1440p)</SelectItem>
                                    <SelectItem value="1080p">Full HD (1080p)</SelectItem>
                                    <SelectItem value="720p">HD (720p)</SelectItem>
                                </SelectContent>
                                <SelectContent v-else-if="downloadFormat === 'mp3'">
                                    <SelectItem value="best">自動（最高音質）</SelectItem>
                                    <SelectItem value="320k">320kbps</SelectItem>
                                    <SelectItem value="256k">256kbps</SelectItem>
                                    <SelectItem value="192k">192kbps</SelectItem>
                                    <SelectItem value="128k">128kbps</SelectItem>
                                </SelectContent>
                                <SelectContent v-else>
                                    <SelectItem value="best">自動（最高ロスレス音質）</SelectItem>
                                </SelectContent>
                            </Select>
                        </div>

                        <div class="flex flex-col gap-2">
                            <Label>保存先フォルダー</Label>
                            <div class="flex gap-2 items-center">
                                <Input :value="displayDownloadDirectory(downloadDir)" readonly class="text-xs grow bg-muted/30" />
                                <Button type="button" variant="outline" size="sm" @click="selectDownloadDir">変更</Button>
                                <Button v-if="downloadDir !== 'default'" type="button" variant="ghost" size="sm" @click="resetDownloadDir">デフォルト</Button>
                            </div>
                        </div>

                        <Button type="button" variant="outline" class="w-full" @click="showAdvancedSettings = !showAdvancedSettings">
                            <IconAdjustments data-icon="inline-start" />
                            詳細設定
                            <span class="text-muted-foreground">字幕・メタデータ・Cookie</span>
                            <IconChevronUp v-if="showAdvancedSettings" data-icon="inline-end" />
                            <IconChevronDown v-else data-icon="inline-end" />
                        </Button>

                        <template v-if="showAdvancedSettings">
                        <div class="flex flex-col gap-2">
                            <Label for="cookie-profile">Cookieの読み込み元</Label>
                            <Select v-model="cookieProfilePath">
                                <SelectTrigger id="cookie-profile" class="w-full">
                                    <SelectValue placeholder="読み込み元を選択" />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem value="none">なし（インポートしない）</SelectItem>
                                    
                                    <SelectGroup v-if="firefoxProfiles.length > 0">
                                        <SelectLabel>Firefox</SelectLabel>
                                        <SelectItem v-for="prof in firefoxProfiles" :key="prof.path" :value="prof.path">
                                            {{ prof.name }} <span v-if="prof.is_default" class="text-muted-foreground text-[10px] ml-1">(デフォルト)</span>
                                        </SelectItem>
                                    </SelectGroup>

                                    <SelectGroup v-if="floorpProfiles.length > 0">
                                        <SelectLabel>Floorp</SelectLabel>
                                        <SelectItem v-for="prof in floorpProfiles" :key="prof.path" :value="prof.path">
                                            {{ prof.name }} <span v-if="prof.is_default" class="text-muted-foreground text-[10px] ml-1">(デフォルト)</span>
                                        </SelectItem>
                                    </SelectGroup>

                                    <SelectGroup v-if="zenProfiles.length > 0">
                                        <SelectLabel>Zen Browser</SelectLabel>
                                        <SelectItem v-for="prof in zenProfiles" :key="prof.path" :value="prof.path">
                                            {{ prof.name }} <span v-if="prof.is_default" class="text-muted-foreground text-[10px] ml-1">(デフォルト)</span>
                                        </SelectItem>
                                    </SelectGroup>
                                </SelectContent>
                            </Select>
                        </div>

                        <div class="flex items-center gap-2 pt-2">
                            <Checkbox id="embed-subtitles" v-model:checked="embedSubtitles" />
                            <Label for="embed-subtitles" class="cursor-pointer font-normal">字幕を埋め込む</Label>
                        </div>

                        <!-- Subtitle detailed options in addTask -->
                        <div v-if="embedSubtitles" class="pl-6 border-l-2 border-primary/20 flex flex-col gap-3 pt-1">
                            <div class="flex items-center gap-2">
                                <Checkbox id="write-auto-subs" v-model:checked="writeAutoSubs" />
                                <Label for="write-auto-subs" class="cursor-pointer text-xs font-normal">自動生成（自動翻訳）の字幕も含める</Label>
                            </div>
                            <div class="flex flex-col gap-1.5">
                                <Label for="sub-langs" class="text-xs">対象の字幕言語</Label>
                                <Select v-model="subLangs">
                                    <SelectTrigger id="sub-langs" class="w-full h-8 text-xs">
                                        <SelectValue placeholder="言語を選択" />
                                    </SelectTrigger>
                                    <SelectContent>
                                        <SelectItem value="ja">日本語</SelectItem>
                                        <SelectItem value="en">英語</SelectItem>
                                        <SelectItem value="all">すべての言語</SelectItem>
                                    </SelectContent>
                                </Select>
                            </div>
                        </div>

                        <div class="flex items-center gap-2">
                            <Checkbox id="embed-thumbnail" v-model:checked="embedThumbnail" />
                            <Label for="embed-thumbnail" class="cursor-pointer font-normal">サムネイルをファイルに埋め込む</Label>
                        </div>

                        <div class="flex items-center gap-2">
                            <Checkbox id="embed-metadata" v-model:checked="embedMetadata" />
                            <Label for="embed-metadata" class="cursor-pointer font-normal">動画のメタデータ（タグ）を埋め込む</Label>
                        </div>
                        </template>
                    </div>
                </div>
                <DialogFooter>
                    <Button :disabled="!hasPlaylistSelection" @click="handleAddTask" type="button">
                        {{ retryingTask ? '再実行' : 'ダウンロードを開始' }}
                    </Button>
                </DialogFooter>
            </template>
        </DialogContent>
    </Dialog>
</template>
