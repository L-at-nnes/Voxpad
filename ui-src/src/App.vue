<template>
  <div class="vx-shell" :class="{ 'vx-shell--tags': currentTab }">
    <BoardRail />
    <TagRail v-if="currentTab" />

    <main class="vx-main">
      <header class="vx-topbar">
        <div class="vx-search">
          <VxIcon name="search" :size="17" />
          <input v-model="query" type="text" :placeholder="$t('app.search')" spellcheck="false" />
          <button
            v-if="query"
            class="vx-search__clear"
            :aria-label="$t('common.clearSearch')"
            @click="query = ''"
          >
            <VxIcon name="close" :size="14" />
          </button>
        </div>
        <div class="vx-topbar__right">
          <ViewModeControl />
          <ZoomControl v-if="viewMode === BoardViewMode.Cards" />
          <button class="vx-icon-btn" :title="$t('app.settings')" @click="openSettings">
            <VxIcon name="settings" :size="19" />
          </button>
        </div>
      </header>

      <template v-if="currentTab">
        <CoverBanner :tab="currentTab" />
        <div v-if="viewTab.sounds.length === 0 && currentTab.sounds.length > 0" class="vx-empty-tag">
          <template v-if="$store.getters.effectiveTag === NO_TAG">
            <h3>{{ $t('tags.noTagEmptyTitle') }}</h3>
            <p>{{ $t('tags.noTagEmptyBody') }}</p>
          </template>
          <template v-else>
            <h3>{{ $t('tags.emptyTitle') }}</h3>
            <p>{{ $t('tags.emptyBody') }}</p>
          </template>
        </div>
        <SoundGrid v-else-if="viewMode === BoardViewMode.Cards" :tab="viewTab" :query="query" />
        <SoundList v-else-if="viewMode === BoardViewMode.List" :tab="viewTab" :query="query" />
        <AlbumView v-else :tab="viewTab" :query="query" />
      </template>
      <div v-else class="vx-empty-shell">
        <div class="vx-empty-shell__art" :style="{ background: emptyGradient }"></div>
        <h2>{{ $t('empty.noBoardTitle') }}</h2>
        <p>{{ $t('empty.noBoardBody') }}</p>
        <button class="vx-btn vx-btn--accent" @click="openCreate">
          <VxIcon name="plus" :size="18" /> {{ $t('app.newBoard') }}
        </button>
      </div>

      <!-- Spotify-style playback bar (only for sounds > 1 min) -->
      <PlaybackBar />
    </main>

    <Inspector />

    <CreateBoardDialog />
    <TagDialog />
    <SettingsDialog />
  </div>
</template>

<script lang="ts">
import Vue from 'vue';
import { initialize } from '@/utils/backend';
import { gradientFromName } from '@/utils/visual';
import { BoardViewMode, Tab } from '@/types';
import { NO_TAG } from '@/store';
import BoardRail from '@/components/BoardRail.vue';
import TagRail from '@/components/TagRail.vue';
import CoverBanner from '@/components/CoverBanner.vue';
import SoundGrid from '@/components/SoundGrid.vue';
import SoundList from '@/components/SoundList.vue';
import AlbumView from '@/components/AlbumView.vue';
import Inspector from '@/components/Inspector.vue';
import ZoomControl from '@/components/ZoomControl.vue';
import ViewModeControl from '@/components/ViewModeControl.vue';
import PlaybackBar from '@/components/PlaybackBar.vue';
import TagDialog from '@/components/TagDialog.vue';
import CreateBoardDialog from '@/components/CreateBoardDialog.vue';
import SettingsDialog from '@/components/SettingsDialog.vue';
import VxIcon from '@/components/VxIcon.vue';

export default Vue.extend({
  name: 'App',
  components: {
    BoardRail, TagRail, CoverBanner, SoundGrid, SoundList, AlbumView, Inspector,
    ZoomControl, ViewModeControl, PlaybackBar, TagDialog, CreateBoardDialog, SettingsDialog, VxIcon,
  },
  data: () => ({ query: '', raf: 0, BoardViewMode, NO_TAG }),
  computed: {
    currentTab(): Tab | undefined {
      return this.$store.getters.currentTab;
    },
    viewTab(): Tab {
      return this.$store.getters.viewTab;
    },
    viewMode(): BoardViewMode {
      return this.$store.getters.effectiveViewMode;
    },
    anyPlaying(): boolean {
      return Object.keys(this.$store.state.tabPlaying).length > 0;
    },
    emptyGradient(): string {
      return gradientFromName('voxpad');
    },
  },
  watch: {
    currentTab() {
      this.query = '';
    },
    anyPlaying(playing: boolean) {
      if (playing && !this.raf) {
        this.tickProgress();
      } else if (!playing && this.raf) {
        cancelAnimationFrame(this.raf);
        this.raf = 0;
      }
    },
  },
  mounted() {
    initialize();
  },
  beforeDestroy() {
    if (this.raf) cancelAnimationFrame(this.raf);
  },
  methods: {
    openSettings() {
      this.$store.commit('setSettingsDialog', true);
    },
    openCreate() {
      this.$store.commit('setCreateBoardDialog', true);
    },
    tickProgress() {
      const tabPlaying = this.$store.state.tabPlaying as Record<number, any>;
      const now = performance.now();
      let still = false;

      for (const tabIdStr of Object.keys(tabPlaying)) {
        const tabId = parseInt(tabIdStr);
        const ps = tabPlaying[tabId];
        if (!ps) continue;
        still = true;
        if (!ps.sound.paused) {
          const len = ps.sound.lengthInMs || ps.sound.sound?.lengthInMs || 0;
          if (len > 0) {
            const ms = ps.progBaseMs + (now - ps.progBaseT);
            this.$store.commit('setTabProgress', {
              tabId,
              progress: Math.min(1, ms / len),
            });
          }
          // len === 0 means unknown duration; PlaybackBar uses rawElapsedMs directly
        }
      }

      if (still) {
        this.raf = requestAnimationFrame(this.tickProgress);
      } else {
        this.raf = 0;
      }
    },
  },
});
</script>

<style scoped lang="scss">
.vx-shell {
  display: grid;
  grid-template-columns: var(--rail-w) 1fr var(--inspector-w);
  height: 100vh;
  background: var(--bg);

  &--tags {
    grid-template-columns: var(--rail-w) var(--tags-w) 1fr var(--inspector-w);
  }
}

.vx-main {
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
}

.vx-topbar {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 14px 20px;
  flex: none;
}

.vx-search {
  display: flex;
  align-items: center;
  gap: 10px;
  flex: 1;
  max-width: 460px;
  height: 40px;
  padding: 0 14px;
  border-radius: 99px;
  background: var(--surface);
  border: 1px solid var(--border);
  color: var(--text-dim);
  transition: border-color 0.14s var(--ease);

  &:focus-within {
    border-color: var(--accent);
  }

  input {
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    outline: none;
    color: var(--text);
    font-size: 14px;
  }
  input::placeholder {
    color: var(--text-dim);
  }
  &__clear {
    border: none;
    background: none;
    color: var(--text-dim);
    display: flex;
  }
}

.vx-topbar__right {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 8px;
}

.vx-empty-tag {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  text-align: center;
  color: var(--text-dim);

  h3 {
    margin: 0;
    color: var(--text);
    font-size: 18px;
  }
  p {
    margin: 0;
    max-width: 320px;
  }
}

.vx-empty-shell {
  margin: auto;
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 12px;
  padding: 40px;

  &__art {
    width: 120px;
    height: 120px;
    border-radius: 28px;
    margin-bottom: 8px;
    opacity: 0.9;
  }
  h2 {
    margin: 0;
    font-size: 22px;
    font-weight: 700;
  }
  p {
    margin: 0 0 8px;
    color: var(--text-dim);
    max-width: 320px;
  }
}
</style>
