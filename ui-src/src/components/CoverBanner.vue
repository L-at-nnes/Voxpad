<template>
  <header class="vx-banner" :style="bannerStyle" @click="deselect">
    <div class="vx-banner__scrim"></div>
    <div class="vx-banner__content">
      <div class="vx-banner__titles">
        <span class="vx-banner__count">{{ soundCount }}</span>
        <h1 class="vx-banner__name">{{ tab.name }}</h1>
      </div>
      <div class="vx-banner__actions">
        <button class="vx-btn" @click="importFiles">
          <VxIcon name="file" :size="17" /> {{ $t('board.importFiles') }}
        </button>
        <button class="vx-btn" @click="importFolder">
          <VxIcon name="folder" :size="17" /> {{ $t('board.importFolder') }}
        </button>
        <button
          v-if="tab.importedFolders && tab.importedFolders.length"
          class="vx-btn"
          :title="$t('board.refreshImportedFolders')"
          @click="refreshImportedFolders"
        >
          <VxIcon name="refresh" :size="17" />
        </button>
        <button class="vx-btn" :title="$t('board.changeCover')" @click="changeCover">
          <VxIcon name="image" :size="17" /> {{ $t('board.cover') }}
        </button>
      </div>
    </div>
  </header>
</template>

<script lang="ts">
import Vue, { PropType } from 'vue';
import { Tab } from '@/types';
import { gradientFromName, fileUrl } from '@/utils/visual';
import VxIcon from '@/components/VxIcon.vue';

export default Vue.extend({
  name: 'CoverBanner',
  components: { VxIcon },
  props: { tab: { type: Object as PropType<Tab>, required: true } },
  computed: {
    soundCount(): string {
      const n = this.tab.sounds.length;
      return `${n} ${this.$t(n === 1 ? 'board.sound' : 'board.sounds')}`;
    },
    bannerStyle(): Record<string, string> {
      if (this.tab.cover) {
        return { backgroundImage: `url("${fileUrl(this.tab.cover)}")` };
      }
      return { background: gradientFromName(this.tab.name) };
    },
  },
  methods: {
    deselect() {
      this.$store.dispatch('selectSound', null);
    },
    importFiles() {
      this.$store.dispatch('importFiles');
    },
    importFolder() {
      this.$store.dispatch('importFolder');
    },
    refreshImportedFolders() {
      this.$store.dispatch('refreshImportedFolders');
    },
    async changeCover() {
      const cover = await this.$store.dispatch('pickCover');
      if (cover) {
        const tab = await window.setTabCover(this.tab.id, cover);
        if (tab) this.$store.commit('replaceTab', tab);
      }
    },
  },
});
</script>

<style scoped lang="scss">
.vx-banner {
  position: relative;
  flex: none;
  margin: 0 20px;
  height: 200px;
  border-radius: var(--radius-lg);
  overflow: hidden;
  background-size: cover;
  background-position: center;
  display: flex;
  align-items: flex-end;

  &__scrim {
    position: absolute;
    inset: 0;
    background: linear-gradient(to top, oklch(0 0 0 / 0.65), oklch(0 0 0 / 0.05) 60%);
  }

  &__content {
    position: relative;
    width: 100%;
    padding: 22px 26px;
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 20px;
  }

  &__count {
    display: block;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: oklch(0.95 0.01 320 / 0.85);
    margin-bottom: 6px;
  }

  &__name {
    margin: 0;
    font-size: 46px;
    font-weight: 800;
    letter-spacing: -0.02em;
    line-height: 1;
    color: oklch(0.99 0.01 320);
    text-shadow: 0 2px 12px oklch(0 0 0 / 0.4);
    word-break: break-word;
  }

  &__actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: none;
  }
}
</style>
