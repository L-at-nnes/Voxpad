<template>
  <nav class="vx-rail" @click="closeMenu">
    <div class="vx-rail__logo" :style="{ background: logoGradient }">V</div>

    <div class="vx-rail__list">
      <BoardAvatar
        v-for="(tab, i) in tabs"
        :key="tab.id"
        :tab="tab"
        :active="i === activeTab"
        @select="select(i)"
        @menu="openMenu($event, tab)"
      />
      <button class="vx-rail__add" :title="$t('app.newBoard')" @click="openCreate">
        <VxIcon name="plus" :size="22" />
      </button>
    </div>

    <transition name="vx-pop">
      <div
        v-if="menu.show"
        class="vx-context"
        :style="{ left: menu.x + 'px', top: menu.y + 'px' }"
        @click.stop
      >
        <button class="vx-context__item vx-context__item--danger" @click="removeMenuTab">
          <VxIcon name="trash" :size="16" /> {{ $t('board.delete') }}
        </button>
      </div>
    </transition>
  </nav>
</template>

<script lang="ts">
import Vue from 'vue';
import { Tab } from '@/types';
import { gradientFromName } from '@/utils/visual';
import BoardAvatar from '@/components/BoardAvatar.vue';
import VxIcon from '@/components/VxIcon.vue';

export default Vue.extend({
  name: 'BoardRail',
  components: { BoardAvatar, VxIcon },
  data: () => ({
    menu: { show: false, x: 0, y: 0, tab: null as Tab | null },
  }),
  computed: {
    tabs(): Tab[] {
      return this.$store.getters.tabs;
    },
    activeTab(): number {
      return this.$store.getters.activeTab;
    },
    logoGradient(): string {
      return gradientFromName('voxpad-app');
    },
  },
  methods: {
    select(i: number) {
      this.$store.dispatch('setActiveTab', i);
    },
    openCreate() {
      this.$store.commit('setCreateBoardDialog', true);
    },
    openMenu(evt: MouseEvent, tab: Tab) {
      this.menu = { show: true, x: Math.min(evt.clientX, window.innerWidth - 180), y: evt.clientY, tab };
    },
    closeMenu() {
      this.menu.show = false;
    },
    removeMenuTab() {
      if (this.menu.tab) this.$store.dispatch('removeBoard', this.menu.tab);
      this.closeMenu();
    },
  },
});
</script>

<style scoped lang="scss">
.vx-rail {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  padding: 14px 0;
  background: var(--surface);
  border-right: 1px solid var(--border);
  overflow: hidden;

  &__logo {
    flex: none;
    width: 42px;
    height: 42px;
    border-radius: 12px;
    display: grid;
    place-items: center;
    font-weight: 800;
    font-size: 22px;
    color: oklch(0.18 0.02 320);
  }

  &__list {
    flex: 1;
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 4px 0;
    // hide scrollbar in the rail, keep scrollability
    scrollbar-width: none;
    &::-webkit-scrollbar {
      display: none;
    }
  }

  &__add {
    flex: none;
    width: 48px;
    height: 48px;
    border-radius: 14px;
    border: 1.5px dashed var(--border);
    background: transparent;
    color: var(--text-dim);
    display: grid;
    place-items: center;
    transition: all 0.14s var(--ease);

    &:hover {
      color: var(--accent);
      border-color: var(--accent);
      background: var(--accent-soft);
    }
  }
}

.vx-context {
  position: fixed;
  z-index: 100;
  min-width: 160px;
  padding: 6px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: 0 12px 32px oklch(0 0 0 / 0.45);

  &__item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 9px 10px;
    border: none;
    background: none;
    border-radius: var(--radius-sm);
    color: var(--text);
    font-size: 13px;
    text-align: left;

    &:hover {
      background: var(--surface-3);
    }
    &--danger {
      color: var(--danger);
    }
    &--danger:hover {
      background: oklch(0.62 0.17 25 / 0.16);
    }
  }
}

.vx-pop-enter-active {
  transition: opacity 0.1s var(--ease), transform 0.1s var(--ease);
}
.vx-pop-enter {
  opacity: 0;
  transform: scale(0.96);
}
</style>
