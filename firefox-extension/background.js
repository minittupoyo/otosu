const YOUTUBE_HOSTS = new Set([
  "youtube.com",
  "www.youtube.com",
  "m.youtube.com",
  "music.youtube.com",
  "youtu.be",
]);

function isYouTubeUrl(value) {
  try {
    const url = new URL(value);
    return url.protocol === "https:" && YOUTUBE_HOSTS.has(url.hostname);
  } catch {
    return false;
  }
}

async function shareWithOtosu(value) {
  if (!isYouTubeUrl(value)) return;
  const deepLink = `otosu://download?url=${encodeURIComponent(value)}`;
  await browser.tabs.create({ url: deepLink });
}

browser.runtime.onInstalled.addListener(() => {
  browser.menus.create({
    id: "share-with-otosu",
    title: "Otosuでダウンロード",
    contexts: ["page", "link", "video", "tab"],
    documentUrlPatterns: ["*://*.youtube.com/*", "*://youtu.be/*"],
  });
});

browser.action.onClicked.addListener((tab) => shareWithOtosu(tab.url));

browser.menus.onClicked.addListener((info, tab) => {
  if (info.menuItemId !== "share-with-otosu") return;
  void shareWithOtosu(info.linkUrl || info.srcUrl || info.pageUrl || tab?.url);
});
