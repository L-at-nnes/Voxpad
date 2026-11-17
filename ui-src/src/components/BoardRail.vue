<template>
  <nav class="vx-rail" @click="closeMenu">
    <div class="vx-rail__logo" :style="{ background: logoGradient }">V</div>

    <div ref="list" class="vx-rail__list">
      <div
        v-for="(tab, i) in tabs"
        :key="tab.id"
        class="vx-rail__row"
        :class="{
          'vx-rail__row--dragging': dragging === tab.id,
          'vx-rail__row--drop-before': dropIndex === i,
          'vx-rail__row--drop-after': dropIndex === tabs.length && i === tabs.length - 1,
        }"
        data-board-row
        @mousedown="pressRow($event, tab.id)"
      >
        <BoardAvatar
          :tab="tab"
          :active="i === activeTab"
          @select="select(i)"
          @menu="openMenu($event, tab)"
        />
      </div>
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
        <button v-if="tabs.length > 1" class="vx-context__item" @click="mergeMenuTab">
          <VxIcon name="grid-view" :size="16" /> {{ $t('board.merge') }}
        </button>
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

// Pointer travel before a press on a board turns into a drag (so plain clicks still select).
const DRAG_THRESHOLD = 5;

export default Vue.extend({
  name: 'BoardRail',
  components: { BoardAvatar, VxIcon },
  data: () => ({
    menu: { show: false, x: 0, y: 0, tab: null as Tab | null },
    // drag-to-reorder state
    pressed: null as { id: number; y: number } | null,
    dragging: null as number | null,
    dropIndex: null as number | null,
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
  beforeDestroy() {
    this.stopDrag();
  },
  methods: {
    select(i: number) {
      // the click that ends a drag must not also select the board
      if (this.dragging !== null) return;
      this.$store.dispatch('setActiveTab', i);
    },

    // --- drag to reorder (pointer events, same approach as the tag rail: native HTML5
    // drag-and-drop is intercepted by the webview's file-drop handling) ---
    pressRow(e: MouseEvent, id: number) {
      if (e.button !== 0) return;
      this.pressed = { id, y: e.clientY };
      window.addEventListener('mousemove', this.onDragMove);
      window.addEventListener('mouseup', this.onDragEnd);
    },
    onDragMove(e: MouseEvent) {
      if (!this.pressed) return;
      if (this.dragging === null) {
        if (Math.abs(e.clientY - this.pressed.y) < DRAG_THRESHOLD) return;
        this.closeMenu();
        this.dragging = this.pressed.id;
      }
      this.dropIndex = this.indexAt(e.clientY);
    },
    // Index of the board the pointer is above (tabs.length = after the last one).
    indexAt(y: number): number {
      const rows = (this.$refs.list as HTMLElement).querySelectorAll('[data-board-row]');
      for (let i = 0; i < rows.length; i++) {
        const r = rows[i].getBoundingClientRect();
        if (y < r.top + r.height / 2) return i;
      }
      return rows.length;
    },
    onDragEnd() {
      const id = this.dragging;
      const to = this.dropIndex;
      this.stopDrag();
      if (id === null || to === null) return;
      const order = this.tabs.map(t => t.id);
      const from = order.indexOf(id);
      order.splice(from, 1);
      order.splice(to > from ? to - 1 : to, 0, id);
      if (order.some((t, i) => t !== this.tabs[i].id)) this.$store.dispatch('reorderBoards', order);
    },
    stopDrag() {
      window.removeEventListener('mousemove', this.onDragMove);
      window.removeEventListener('mouseup', this.onDragEnd);
      this.pressed = null;
      this.dropIndex = null;
      // keep `dragging` set until after the trailing click event so select() can ignore it
      setTimeout(() => (this.dragging = null), 0);
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
    mergeMenuTab() {
      if (this.menu.tab) this.$store.commit('setMergeBoard', this.menu.tab.id);
      this.closeMenu();
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

  &__row {
    position: relative;
    flex: none;
    display: flex;
    justify-content: center;
    width: 100%;

    &--dragging {
      opacity: 0.45;
    }
    // drop indicator line
    &--drop-before::before,
    &--drop-after::after {
      content: '';
      position: absolute;
      left: 10px;
      right: 10px;
      height: 3px;
      border-radius: 2px;
      background: var(--accent);
      z-index: 1;
    }
    &--drop-before::before {
      top: -3px;
    }
    &--drop-after::after {
      bottom: -3px;
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
