<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { Dialog, DialogContent, DialogClose, DialogHeader, DialogFooter, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectGroup, SelectItem, SelectLabel, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Label } from "@/components/ui/label";
import { Checkbox } from "@/components/ui/checkbox";
import { Separator } from "@/components/ui/separator";
import { globalSettings, resetGlobalSettings } from "@/lib/globalSettings";
import { displayDownloadDirectory } from "@/lib/downloadDirectory";
import { invoke } from "@tauri-apps/api/core";
import { IconSettings } from "@tabler/icons-vue";
import { open } from '@tauri-apps/plugin-dialog';

interface ProfileInfo {
    name: string;
    path: string;
    is_default: boolean;
}

const isOpen = ref(false);

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

        if (globalSettings.value.defaultCookieProfilePath !== 'none') {
            const exists = [
                ...firefoxProfiles.value,
                ...floorpProfiles.value,
                ...zenProfiles.value
            ].some(p => p.path === globalSettings.value.defaultCookieProfilePath);
            if (!exists) {
                globalSettings.value.defaultCookieProfilePath = 'none';
            }
        }
    } catch (err) {
        console.error(err);
    }
};

const selectDirectory = async () => {
    try {
        const selected = await open({
            directory: true,
            multiple: false,
            defaultPath: globalSettings.value.defaultDownloadDir !== 'default' ? globalSettings.value.defaultDownloadDir : undefined
        });
        if (selected && typeof selected === 'string') {
            globalSettings.value.defaultDownloadDir = selected;
        }
    } catch (err) {
        console.error(err);
    }
};

const resetDirectory = () => {
    globalSettings.value.defaultDownloadDir = 'default';
};

const handleReset = () => {
    if (confirm('すべての設定を初期値に戻しますか？')) {
        resetGlobalSettings();
        loadAllProfiles();
    }
};

watch(isOpen, (newVal) => {
    if (newVal) {
        loadAllProfiles();
    }
});

watch(() => globalSettings.value.defaultFormat, () => {
    globalSettings.value.defaultQuality = 'best';
});

onMounted(() => {
    loadAllProfiles();
});
</script>

<template>
    <Dialog v-model:open="isOpen">
        <DialogTrigger as-child>
            <Button variant="ghost" size="icon">
                <IconSettings class="size-5 text-muted-foreground" />
            </Button>
        </DialogTrigger>
        <DialogContent class="sm:max-w-[425px]">
            <DialogHeader>
                <DialogTitle>グローバル設定</DialogTitle>
            </DialogHeader>

            <div class="grid gap-4 py-4 max-h-[70vh] overflow-y-auto pr-2">
                <!-- Cookie Settings -->
                <div class="flex flex-col gap-3">
                    <h3 class="text-sm font-semibold">Cookie インポート設定</h3>
                    <div class="flex flex-col gap-2">
                        <Label for="default-cookie-profile">デフォルトの Cookie 読み込み元</Label>
                        <Select v-model="globalSettings.defaultCookieProfilePath">
                            <SelectTrigger id="default-cookie-profile" class="w-full">
                                <SelectValue placeholder="プロファイルを選択" />
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
                </div>

                <Separator />

                <!-- Download Defaults -->
                <div class="flex flex-col gap-3">
                    <h3 class="text-sm font-semibold">ダウンロードのデフォルト設定</h3>
                    <div class="flex flex-col gap-2">
                        <Label for="default-format">デフォルト形式</Label>
                        <Select v-model="globalSettings.defaultFormat">
                            <SelectTrigger id="default-format" class="w-full">
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
                        <Label for="default-quality">デフォルト品質</Label>
                        <Select v-model="globalSettings.defaultQuality">
                            <SelectTrigger id="default-quality" class="w-full">
                                <SelectValue placeholder="品質を選択" />
                            </SelectTrigger>
                            <SelectContent v-if="globalSettings.defaultFormat === 'mp4' || globalSettings.defaultFormat === 'mkv'">
                                <SelectItem value="best">自動（最高画質）</SelectItem>
                                <SelectItem value="2160p">4K (2160p)</SelectItem>
                                <SelectItem value="1440p">2K (1440p)</SelectItem>
                                <SelectItem value="1080p">Full HD (1080p)</SelectItem>
                                <SelectItem value="720p">HD (720p)</SelectItem>
                            </SelectContent>
                            <SelectContent v-else-if="globalSettings.defaultFormat === 'mp3'">
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

                    <!-- Custom Download Directory -->
                    <div class="flex flex-col gap-2">
                        <Label>デフォルトの保存先フォルダー</Label>
                        <div class="flex gap-2 items-center">
                            <Input :value="displayDownloadDirectory(globalSettings.defaultDownloadDir)" readonly class="text-xs grow bg-muted/30" />
                            <Button type="button" variant="outline" size="sm" @click="selectDirectory">選択</Button>
                            <Button v-if="globalSettings.defaultDownloadDir !== 'default'" type="button" variant="ghost" size="sm" @click="resetDirectory">リセット</Button>
                        </div>
                    </div>

                    <div class="flex flex-col gap-3 pt-2">
                        <div class="flex items-center gap-2">
                            <Checkbox id="default-embed-subtitles" v-model:checked="globalSettings.defaultEmbedSubtitles" />
                            <Label for="default-embed-subtitles" class="cursor-pointer font-normal">字幕を埋め込む</Label>
                        </div>

                        <!-- Subtitle detailed options -->
                        <div v-if="globalSettings.defaultEmbedSubtitles" class="pl-6 border-l-2 border-primary/20 flex flex-col gap-3 pt-1">
                            <div class="flex items-center gap-2">
                                <Checkbox id="default-write-auto-subs" v-model:checked="globalSettings.defaultWriteAutoSubs" />
                                <Label for="default-write-auto-subs" class="cursor-pointer text-xs font-normal">自動生成（自動翻訳）の字幕も含める</Label>
                            </div>
                            <div class="flex flex-col gap-1.5">
                                <Label for="default-sub-langs" class="text-xs">対象の字幕言語</Label>
                                <Select v-model="globalSettings.defaultSubLangs">
                                    <SelectTrigger id="default-sub-langs" class="w-full h-8 text-xs">
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
                            <Checkbox id="default-embed-thumbnail" v-model:checked="globalSettings.defaultEmbedThumbnail" />
                            <Label for="default-embed-thumbnail" class="cursor-pointer font-normal">サムネイルをファイルに埋め込む</Label>
                        </div>

                        <div class="flex items-center gap-2">
                            <Checkbox id="default-embed-metadata" v-model:checked="globalSettings.defaultEmbedMetadata" />
                            <Label for="default-embed-metadata" class="cursor-pointer font-normal">動画のメタデータ（タグ）を埋め込む</Label>
                        </div>

                        <div class="flex flex-col gap-2">
                            <Label for="default-playlist-behavior">プレイリストURLのダウンロード挙動</Label>
                            <Select v-model="globalSettings.defaultPlaylistBehavior">
                                <SelectTrigger id="default-playlist-behavior" class="w-full">
                                    <SelectValue placeholder="挙動を選択" />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem value="no">単一の動画のみダウンロード</SelectItem>
                                    <SelectItem value="yes">プレイリスト全体をダウンロード</SelectItem>
                                </SelectContent>
                            </Select>
                        </div>
                    </div>
                </div>
            </div>

            <DialogFooter class="flex flex-row justify-between items-center w-full gap-2">
                <Button type="button" variant="destructive" size="sm" @click="handleReset">設定を初期化</Button>
                <DialogClose as-child>
                    <Button type="button" size="sm">閉じる</Button>
                </DialogClose>
            </DialogFooter>
        </DialogContent>
    </Dialog>
</template>
