<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Tabs, TabsTrigger, TabsList, TabsContent } from "@/components/ui/tabs";
import { Item, ItemContent, ItemTitle, ItemDescription, ItemMedia } from "@/components/ui/item";
import { Progress } from "@/components/ui/progress";
import AddTask from "./components/addTask.vue";
import settings from "./components/settings.vue";
import { tasks, initTaskListener, type DownloadTask } from "@/lib/taskState";
import { IconVideo, IconDownloadOff } from "@tabler/icons-vue";
import { Button } from "@/components/ui/button";

interface DependencyStatus {
    installed: boolean;
    version?: string;
    message?: string;
}

interface DependencyReport {
    yt_dlp: DependencyStatus;
    ffmpeg: DependencyStatus;
    deno: DependencyStatus;
}

const dependencyReport = ref<DependencyReport | null>(null);
const checkingDependencies = ref(false);
const addTaskDialog = ref<InstanceType<typeof AddTask> | null>(null);
const retryDownload = (task: DownloadTask) => addTaskDialog.value?.openForRetry(task);
const handleSharedUrl = (event: Event) => {
    const sharedUrl = (event as CustomEvent<string>).detail;
    if (sharedUrl) void addTaskDialog.value?.openWithUrl(sharedUrl);
};
const missingDependencies = computed(() => {
    const report = dependencyReport.value;
    if (!report) return [];
    return [
        !report.yt_dlp.installed ? `yt-dlp: ${report.yt_dlp.message || '自動セットアップに失敗しました'}` : null,
        !report.ffmpeg.installed ? 'ffmpeg: 動画・音声の変換や結合に必要です' : null,
        !report.deno.installed ? `Deno: ${report.deno.message || 'YouTubeのJavaScript処理に2.3以上が推奨されます'}` : null,
    ].filter((message): message is string => message !== null);
});

const checkDependencies = async () => {
    checkingDependencies.value = true;
    try {
        dependencyReport.value = await invoke<DependencyReport>('setup_dependencies');
    } catch (error) {
        dependencyReport.value = {
            yt_dlp: { installed: false, message: String(error) },
            ffmpeg: { installed: false },
            deno: { installed: false },
        };
    } finally {
        checkingDependencies.value = false;
    }
};

const activeTasks = computed(() => tasks.value.filter(t => t.status === 'downloading'));
const doneTasks = computed(() => tasks.value.filter(t => t.status === 'done'));
const failedTasks = computed(() => tasks.value.filter(t => t.status === 'failed'));

onMounted(() => {
    initTaskListener();
    void checkDependencies();
    window.addEventListener('otosu-share-url', handleSharedUrl);
});

onUnmounted(() => window.removeEventListener('otosu-share-url', handleSharedUrl));
</script>

<template>
    <div class="flex flex-col font-sans min-h-screen bg-background">
        <header class="flex sticky w-full items-center top-0 z-50 bg-background/95 backdrop-blur border-b px-4">
            <div class="flex flex-row w-full items-center justify-between h-16">
                <h1 class="text-xl font-bold">Otosu</h1>
                <div class="flex flex-row items-center gap-2">
                    <settings />
                    <AddTask ref="addTaskDialog" />
                </div>
            </div>
        </header>
        <main class="grow p-4 max-w-4xl mx-auto w-full">
            <Item v-if="missingDependencies.length" variant="outline" class="mb-4">
                <ItemContent>
                    <ItemTitle>実行環境の確認が必要です</ItemTitle>
                    <ItemDescription>
                        <span v-for="message in missingDependencies" :key="message" class="block">{{ message }}</span>
                    </ItemDescription>
                </ItemContent>
                <Button type="button" variant="outline" size="sm" :disabled="checkingDependencies" @click="checkDependencies">
                    再確認
                </Button>
            </Item>
            <Tabs default-value="all-task" class="w-full">
                <TabsList class="w-full grid grid-cols-4 mb-6">
                    <TabsTrigger value="all-task">すべて</TabsTrigger>
                    <TabsTrigger value="active-task">実行中 ({{ activeTasks.length }})</TabsTrigger>
                    <TabsTrigger value="done-task">完了 ({{ doneTasks.length }})</TabsTrigger>
                    <TabsTrigger value="failed-task">エラー ({{ failedTasks.length }})</TabsTrigger>
                </TabsList>

                <TabsContent value="all-task" class="space-y-4">
                    <div v-if="tasks.length === 0" class="flex flex-col items-center justify-center py-20 text-muted-foreground border border-dashed rounded-lg bg-card/50">
                        <IconDownloadOff class="size-12 mb-3 opacity-40 text-muted-foreground" />
                        <p class="text-sm font-medium">ダウンロード履歴はありません</p>
                        <p class="text-xs text-muted-foreground mt-1">「ダウンロードを追加」から開始できます</p>
                    </div>
                    <div v-else class="flex flex-col gap-3">
                        <Item v-for="task in tasks" :key="task.id" variant="outline" class="w-full flex-col items-stretch gap-2 bg-card p-4 transition-all hover:border-muted-foreground/30">
                            <div class="flex items-center gap-4 w-full">
                                <ItemMedia :variant="task.thumbnail ? 'image' : 'icon'" :style="task.thumbnail ? 'width: 96px; height: auto; aspect-ratio: 16/9;' : ''">
                                    <img v-if="task.thumbnail" :src="task.thumbnail" alt="Thumbnail" />
                                    <IconVideo v-else class="size-4" />
                                </ItemMedia>
                                <ItemContent class="min-w-0">
                                    <ItemTitle class="truncate block max-w-full font-semibold">{{ task.title }}</ItemTitle>
                                    <ItemDescription class="text-xs">{{ task.duration }} • {{ ['mp4', 'mkv'].includes(task.format) ? '動画' : '音声' }} ({{ task.quality }})</ItemDescription>
                                </ItemContent>
                                <div class="text-right shrink-0">
                                    <span v-if="task.status === 'downloading'" class="text-xs font-semibold text-primary animate-pulse">ダウンロード中</span>
                                    <span v-else-if="task.status === 'done'" class="text-xs font-semibold text-emerald-500">完了</span>
                                    <span v-else-if="task.status === 'failed'" class="text-xs font-semibold text-destructive">失敗</span>
                                </div>
                            </div>

                            <div v-if="task.status === 'downloading'" class="w-full mt-2">
                                <Progress :model-value="task.progress" class="h-1.5" />
                                <div class="flex justify-between items-center text-[10px] text-muted-foreground mt-1">
                                    <span>{{ task.progress.toFixed(1) }}%</span>
                                    <span v-if="task.speed">{{ task.speed }} (ETA: {{ task.eta }})</span>
                                </div>
                            </div>
                            <div v-else-if="task.status === 'failed'" class="w-full mt-1 text-xs text-destructive bg-destructive/5 p-2 rounded-sm border border-destructive/20 whitespace-pre-wrap">
                                {{ task.error }}
                            </div>
                            <div v-if="task.status === 'failed'" class="flex justify-end w-full">
                                <Button type="button" variant="outline" size="sm" @click="retryDownload(task)">設定して再実行</Button>
                            </div>
                        </Item>
                    </div>
                </TabsContent>

                <TabsContent value="active-task" class="space-y-4">
                    <div v-if="activeTasks.length === 0" class="flex flex-col items-center justify-center py-20 text-muted-foreground border border-dashed rounded-lg bg-card/50">
                        <IconDownloadOff class="size-12 mb-3 opacity-40" />
                        <p class="text-sm font-medium">アクティブなタスクはありません</p>
                    </div>
                    <div v-else class="flex flex-col gap-3">
                        <Item v-for="task in activeTasks" :key="task.id" variant="outline" class="w-full flex-col items-stretch gap-2 bg-card p-4 transition-all hover:border-muted-foreground/30">
                            <div class="flex items-center gap-4 w-full">
                                <ItemMedia :variant="task.thumbnail ? 'image' : 'icon'" :style="task.thumbnail ? 'width: 96px; height: auto; aspect-ratio: 16/9;' : ''">
                                    <img v-if="task.thumbnail" :src="task.thumbnail" alt="Thumbnail" />
                                    <IconVideo v-else class="size-4" />
                                </ItemMedia>
                                <ItemContent class="min-w-0">
                                    <ItemTitle class="truncate block max-w-full font-semibold">{{ task.title }}</ItemTitle>
                                    <ItemDescription class="text-xs">{{ task.duration }} • {{ ['mp4', 'mkv'].includes(task.format) ? '動画' : '音声' }} ({{ task.quality }})</ItemDescription>
                                </ItemContent>
                                <div class="text-right shrink-0">
                                    <span class="text-xs font-semibold text-primary animate-pulse">ダウンロード中</span>
                                </div>
                            </div>

                            <div class="w-full mt-2">
                                <Progress :model-value="task.progress" class="h-1.5" />
                                <div class="flex justify-between items-center text-[10px] text-muted-foreground mt-1">
                                    <span>{{ task.progress.toFixed(1) }}%</span>
                                    <span v-if="task.speed">{{ task.speed }} (ETA: {{ task.eta }})</span>
                                </div>
                            </div>
                        </Item>
                    </div>
                </TabsContent>

                <TabsContent value="done-task" class="space-y-4">
                    <div v-if="doneTasks.length === 0" class="flex flex-col items-center justify-center py-20 text-muted-foreground border border-dashed rounded-lg bg-card/50">
                        <IconDownloadOff class="size-12 mb-3 opacity-40" />
                        <p class="text-sm font-medium">完了済みのタスクはありません</p>
                    </div>
                    <div v-else class="flex flex-col gap-3">
                        <Item v-for="task in doneTasks" :key="task.id" variant="outline" class="w-full flex-col items-stretch gap-2 bg-card p-4 transition-all hover:border-muted-foreground/30">
                            <div class="flex items-center gap-4 w-full">
                                <ItemMedia :variant="task.thumbnail ? 'image' : 'icon'" :style="task.thumbnail ? 'width: 96px; height: auto; aspect-ratio: 16/9;' : ''">
                                    <img v-if="task.thumbnail" :src="task.thumbnail" alt="Thumbnail" />
                                    <IconVideo v-else class="size-4" />
                                </ItemMedia>
                                <ItemContent class="min-w-0">
                                    <ItemTitle class="truncate block max-w-full font-semibold">{{ task.title }}</ItemTitle>
                                    <ItemDescription class="text-xs">{{ task.duration }} • {{ ['mp4', 'mkv'].includes(task.format) ? '動画' : '音声' }} ({{ task.quality }})</ItemDescription>
                                </ItemContent>
                                <div class="text-right shrink-0">
                                    <span class="text-xs font-semibold text-emerald-500">完了</span>
                                </div>
                            </div>
                        </Item>
                    </div>
                </TabsContent>

                <TabsContent value="failed-task" class="space-y-4">
                    <div v-if="failedTasks.length === 0" class="flex flex-col items-center justify-center py-20 text-muted-foreground border border-dashed rounded-lg bg-card/50">
                        <IconDownloadOff class="size-12 mb-3 opacity-40" />
                        <p class="text-sm font-medium">失敗したタスクはありません</p>
                    </div>
                    <div v-else class="flex flex-col gap-3">
                        <Item v-for="task in failedTasks" :key="task.id" variant="outline" class="w-full flex-col items-stretch gap-2 bg-card p-4 transition-all hover:border-muted-foreground/30">
                            <div class="flex items-center gap-4 w-full">
                                <ItemMedia :variant="task.thumbnail ? 'image' : 'icon'" :style="task.thumbnail ? 'width: 96px; height: auto; aspect-ratio: 16/9;' : ''">
                                    <img v-if="task.thumbnail" :src="task.thumbnail" alt="Thumbnail" />
                                    <IconVideo v-else class="size-4" />
                                </ItemMedia>
                                <ItemContent class="min-w-0">
                                    <ItemTitle class="truncate block max-w-full font-semibold">{{ task.title }}</ItemTitle>
                                    <ItemDescription class="text-xs">{{ task.duration }} • {{ ['mp4', 'mkv'].includes(task.format) ? '動画' : '音声' }} ({{ task.quality }})</ItemDescription>
                                </ItemContent>
                                <div class="text-right shrink-0">
                                    <span class="text-xs font-semibold text-destructive">失敗</span>
                                </div>
                            </div>
                            <div class="w-full mt-1 text-xs text-destructive bg-destructive/5 p-2 rounded-sm border border-destructive/20 whitespace-pre-wrap">
                                {{ task.error }}
                            </div>
                            <div class="flex justify-end w-full">
                                <Button type="button" variant="outline" size="sm" @click="retryDownload(task)">設定して再実行</Button>
                            </div>
                        </Item>
                    </div>
                </TabsContent>
            </Tabs>
        </main>
    </div>
</template>
