import $store, { findTabForSound } from '@/store';
import i18n from '@/plugins/i18n';
import Vue from 'vue';
import { Sound } from '@/types';

export async function openUrl(url: string): Promise<void> {
  await window.openUrl(url);
}

export async function initialize(): Promise<void> {
  window.onError = (error: number) => {
    Vue.$toast.error(`${i18n.t('errors.error')}: ${i18n.t('errors.' + error)}`);
  };

  window.getTranslation = path => `${i18n.t(path)}`;
  window.getStore = () => $store;

  window.updateSound = playingSound => {
    const tabId = findTabForSound(playingSound.sound.id);
    if (tabId != null) {
      $store.commit('updateTabSound', {
        tabId,
        ms: playingSound.readInMs,
        paused: playingSound.paused,
      });
    }
  };

  window.finishSound = playingSound => {
    // Resolve the board the finished sound actually belongs to — NOT the currently-viewed
    // tab — so a board playing in the background (queue/shuffle) keeps cycling correctly even
    // while the user is looking at a different soundboard.
    const tabId = findTabForSound(playingSound.sound.id);
    if (tabId == null) return;

    const queue = $store.state.tabQueue[tabId] || [];
    const shuffling = !!$store.state.tabShuffle[tabId];

    if (queue.length > 0) {
      // Keep the bar visible through the transition — playNextInQueue will replace
      // the tabPlaying entry directly without a clear-then-set flash.
      const history = $store.state.tabHistory[tabId] || [];
      $store.commit('setTabHistory', { tabId, history: [...history, playingSound.sound] });
      $store.dispatch('playNextInQueue', tabId);
    } else if (shuffling) {
      const history = $store.state.tabHistory[tabId] || [];
      $store.commit('setTabHistory', { tabId, history: [...history, playingSound.sound] });
      $store.dispatch('playRandomNext', playingSound.sound);
    } else {
      $store.commit('clearTabPlaying', tabId);
    }
  };

  window.onSoundPlayed = playingSound => {
    if (!playingSound) return;
    const tabId = findTabForSound(playingSound.sound.id);
    if (tabId != null) {
      $store.commit('setTabPlaying', { tabId, sound: playingSound });
    }
  };

  window.hotkeyReceived = (_seq: string, hotkeyData: number[]) => {
    const soundId = $store.getters.capturingHotkeyFor as number | null;
    if (soundId === null) return;
    const tab = $store.getters.currentTab;
    const sound: Sound | undefined = tab && tab.sounds.find((s: Sound) => s.id === soundId);
    if (sound) {
      $store.dispatch('setHotkeys', { sound, hotkeys: hotkeyData });
    }
    window.requestHotkey(false);
    $store.commit('setCapturingHotkeyFor', null);
  };

  await Promise.all([
    $store.dispatch('getSettings'),
    $store.dispatch('getIsLinux'),
    $store.dispatch('getTabs'),
  ]);

  const saved = $store.state.settings.selectedTab;
  if (saved >= 0 && saved < $store.state.tabs.length) {
    $store.commit('setActiveTab', saved);
  }

  await $store.dispatch('getOutputs', $store.state.settings.outputs);
}
