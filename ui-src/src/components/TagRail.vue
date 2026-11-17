<template>
  <nav ref="nav" class="vx-tags">
    <div class="vx-tags__head">
      <span>{{ $t('tags.title') }}</span>
      <button class="vx-tags__add" :title="$t('tags.create')" @click="startCreate">
        <VxIcon name="plus" :size="18" />
      </button>
    </div>

    <div ref="list" class="vx-tags__list">
      <button
        class="vx-tags__item"
        :class="{ 'vx-tags__item--active': !activeTag }"
        @click="select(null)"
      >
        <VxIcon name="grid-view" :size="18" />
        <span class="vx-tags__name">{{ $t('tags.all') }}</span>
        <span class="vx-tags__count">{{ total }}</span>
      </button>

      <button
        v-if="untagged > 0"
        class="vx-tags__item"
        :class="{ 'vx-tags__item--active': activeTag === NO_TAG }"
        @click="select(NO_TAG)"
      >
        <VxIcon name="tag" :size="18" />
        <span class="vx-tags__name">{{ $t('tags.noTag') }}</span>
        <span class="vx-tags__count">{{ untagged }}</span>
      </button>

      <input
        v-if="creating"
        ref="createInput"
        v-model="draft"
        class="vx-tags__input"
        type="text"
        :placeholder="$t('tags.newTag')"
        maxlength="32"
        spellcheck="false"
        @keydown.enter.prevent="commitCreate"
        @keydown.esc.stop="cancel"
        @blur="commitCreate"
      />

      <div
        v-for="(t, i) in tags"
        :key="t.tag"
        class="vx-tags__row"
        :class="{
          'vx-tags__row--dragging': dragging === t.tag,
          'vx-tags__row--drop-before': dropIndex === i,
          'vx-tags__row--drop-after': dropIndex === tags.length && i === tags.length - 1,
        }"
        data-tag-row
        :data-tag-drop="t.tag"
        @contextmenu.prevent.stop="openMenu($event, t.tag)"
      >
        <input
          v-if="editing === t.tag"
          ref="renameInput"
          v-model="draft"
          class="vx-tags__input"
          type="text"
          maxlength="32"
          spellcheck="false"
          @keydown.enter.prevent="commitRename"
          @keydown.esc.stop="cancel"
          @blur="commitRename"
        />

        <template v-else>
          <button
            class="vx-tags__item"
            :class="{ 'vx-tags__item--active': activeTag === t.tag }"
            :title="t.tag"
            @mousedown="pressRow($event, t.tag)"
            @click="select(t.tag)"
          >
            <VxIcon name="tag" :size="18" />
            <span class="vx-tags__name">{{ t.tag }}</span>
            <span class="vx-tags__count">{{ t.count }}</span>
          </button>

          <div v-if="confirming === t.tag" class="vx-tags__actions vx-tags__actions--open">
            <span class="vx-tags__confirm">{{ $t('tags.deleteConfirm') }}</span>
            <button class="vx-tags__act vx-tags__act--danger" :title="$t('tags.delete')" @click="remove(t.tag)">
              <VxIcon name="check" :size="15" />
            </button>
            <button class="vx-tags__act" :title="$t('common.cancel')" @click="confirming = null">
              <VxIcon name="close" :size="15" />
            </button>
          </div>
          <div v-else class="vx-tags__actions">
            <button class="vx-tags__act" :title="$t('tags.rename')" @click="startRename(t.tag)">
              <VxIcon name="edit" :size="15" />
            </button>
            <button class="vx-tags__act" :title="$t('tags.delete')" @click="confirming = t.tag">
              <VxIcon name="trash" :size="15" />
            </button>
          </div>
        </template>
      </div>
    </div>

    <div class="vx-tags__resize" @mousedown.prevent="startResize"></div>

    <transition name="vx-pop">
      <div
        v-if="menu.show"
        class="vx-context"
        :style="{ left: menu.x + 'px', top: menu.y + 'px' }"
        @click.stop
        @contextmenu.prevent.stop
      >
        <button class="vx-context__item" @click="openSplit">
          <VxIcon name="grid-view" :size="16" /> {{ $t('tags.toBoard') }}
        </button>
        <template v-if="otherBoards.length">
          <div class="vx-context__label">{{ $t('tags.moveTo') }}</div>
          <div class="vx-context__boards">
            <button
              v-for="b in otherBoards"
              :key="b.id"
              class="vx-context__item"
              :title="b.name"
              @click="moveMenuTag(b.id)"
            >
              <span class="vx-context__name">{{ b.name }}</span>
            </button>
          </div>
        </template>
      </div>
    </transition>

    <VxModal :value="splitOpen" :title="$t('tags.toBoardTitle')" @input="closeSplit">
      <p class="vx-split__hint">{{ $t('tags.toBoardHint', { tag: splitTag, count: splitCount }) }}</p>
      <label class="vx-split__label">{{ $t('create.name') }}</label>
      <input
        ref="splitInput"
        v-model="boardName"
        class="vx-input"
        type="text"
        maxlength="48"
        spellcheck="false"
        @keydown.enter="confirmSplit"
      />
      <template #footer>
        <button class="vx-btn" @click="closeSplit">{{ $t('common.cancel') }}</button>
        <button class="vx-btn vx-btn--accent" :disabled="!boardName.trim()" @click="confirmSplit">
          {{ $t('tags.toBoardConfirm') }}
        </button>
      </template>
    </VxModal>
  </nav>
</template>

<script lang="ts">
import Vue from 'vue';
import VxIcon from '@/components/VxIcon.vue';
import VxModal from '@/components/VxModal.vue';
import { Tab } from '@/types';
import { NO_TAG } from '@/store';

const WIDTH_KEY = 'voxpad.tagRailWidth';
const MIN_WIDTH = 150;
const MAX_WIDTH = 440;
// Pointer travel before a press on a tag turns into a drag (so plain clicks still select).
const DRAG_THRESHOLD = 5;

function applyWidth(px: number): void {
  document.documentElement.style.setProperty('--tags-w', `${px}px`);
}

export default Vue.extend({
  name: 'TagRail',
  components: { VxIcon, VxModal },
  data: () => ({
    NO_TAG,
    creating: false,
    editing: null as string | null,
    confirming: null as string | null,
    draft: '',
    // drag-to-reorder state
    pressed: null as { tag: string; y: number } | null,
    dragging: null as string | null,
    dropIndex: null as number | null,
    // board under the pointer while dragging (dropping there moves the tag to that board)
    dropBoard: null as number | null,
    // right-click menu and the "turn into a soundboard" dialog
    menu: { show: false, x: 0, y: 0, tag: '' },
    splitOpen: false,
    splitTag: '',
    boardName: '',
  }),
  computed: {
    tags(): { tag: string; count: number }[] {
      return this.$store.getters.tagCounts;
    },
    activeTag(): string | null {
      return this.$store.getters.effectiveTag;
    },
    untagged(): number {
      return this.$store.getters.untaggedCount;
    },
    total(): number {
      const tab = this.$store.getters.currentTab;
      return tab ? tab.sounds.length : 0;
    },
    otherBoards(): Tab[] {
      const current = this.$store.getters.currentTab as Tab | null;
      return this.$store.getters.tabs.filter((t: Tab) => !current || t.id !== current.id);
    },
    splitCount(): number {
      return this.tags.find(t => t.tag === this.splitTag)?.count ?? 0;
    },
  },
  created() {
    try {
      const saved = parseInt(localStorage.getItem(WIDTH_KEY) || '', 10);
      if (saved >= MIN_WIDTH && saved <= MAX_WIDTH) applyWidth(saved);
    } catch {
      // default width from the stylesheet
    }
  },
  mounted() {
    document.addEventListener('click', this.closeMenu);
    document.addEventListener('contextmenu', this.closeMenu);
    window.addEventListener('keydown', this.onKey);
  },
  beforeDestroy() {
    document.removeEventListener('click', this.closeMenu);
    document.removeEventListener('contextmenu', this.closeMenu);
    window.removeEventListener('keydown', this.onKey);
    this.stopDrag();
    this.stopResize();
  },
  methods: {
    select(tag: string | null) {
      // the click that ends a drag must not also select the tag
      if (this.dragging) return;
      this.$store.commit('setActiveTag', tag);
    },
    focus(ref: string) {
      this.$nextTick(() => {
        const el = this.$refs[ref] as HTMLInputElement | HTMLInputElement[] | undefined;
        (Array.isArray(el) ? el[0] : el)?.focus();
      });
    },
    cancel() {
      this.creating = false;
      this.editing = null;
      this.draft = '';
    },
    startCreate() {
      this.cancel();
      this.confirming = null;
      this.creating = true;
      this.focus('createInput');
    },
    commitCreate() {
      if (!this.creating) return;
      const name = this.draft.trim();
      this.cancel();
      if (name) this.$store.dispatch('createTag', name);
    },
    startRename(tag: string) {
      this.cancel();
      this.confirming = null;
      this.editing = tag;
      this.draft = tag;
      this.focus('renameInput');
    },
    commitRename() {
      const old = this.editing;
      if (old === null) return;
      const name = this.draft.trim();
      this.cancel();
      if (name && name !== old) this.$store.dispatch('renameTag', { old, name });
    },
    remove(tag: string) {
      this.confirming = null;
      this.$store.dispatch('deleteTag', tag);
    },

    // --- right-click menu: turn a tag into a soundboard ---
    openMenu(e: MouseEvent, tag: string) {
      this.cancel();
      this.confirming = null;
      this.menu = {
        show: true,
        x: Math.min(e.clientX, window.innerWidth - 220),
        y: Math.min(e.clientY, window.innerHeight - 320),
        tag,
      };
    },
    closeMenu() {
      this.menu.show = false;
    },
    onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') this.closeMenu();
    },
    openSplit() {
      this.splitTag = this.menu.tag;
      this.boardName = this.menu.tag;
      this.closeMenu();
      this.splitOpen = true;
      this.$nextTick(() => {
        const input = this.$refs.splitInput as HTMLInputElement | undefined;
        input?.focus();
        input?.select();
      });
    },
    async moveMenuTag(dest: number) {
      const tag = this.menu.tag;
      this.closeMenu();
      await this.moveTag(tag, dest);
    },
    async moveTag(tag: string, dest: number) {
      const board = (this.$store.getters.tabs as Tab[]).find(t => t.id === dest);
      const done = await this.$store.dispatch('moveTag', { tag, dest });
      if (done && board) Vue.$toast.success(`${this.$t('tags.moved', { tag, name: board.name })}`);
    },
    closeSplit() {
      this.splitOpen = false;
    },
    async confirmSplit() {
      const name = this.boardName.trim();
      if (!name || !this.splitOpen) return;
      this.splitOpen = false;
      const done = await this.$store.dispatch('tagToBoard', { tag: this.splitTag, name });
      if (done) Vue.$toast.success(`${this.$t('tags.toBoardDone', { name })}`);
    },

    // --- drag to reorder (pointer events: native HTML5 drag-and-drop is intercepted by the
    // webview's file-drop handling) ---
    pressRow(e: MouseEvent, tag: string) {
      if (e.button !== 0) return;
      this.pressed = { tag, y: e.clientY };
      window.addEventListener('mousemove', this.onDragMove);
      window.addEventListener('mouseup', this.onDragEnd);
    },
    onDragMove(e: MouseEvent) {
      if (!this.pressed) return;
      if (!this.dragging) {
        if (Math.abs(e.clientY - this.pressed.y) < DRAG_THRESHOLD) return;
        this.dragging = this.pressed.tag;
      }
      // over a soundboard in the left rail: dropping moves the tag there instead of reordering
      const over = document.elementFromPoint(e.clientX, e.clientY)?.closest('[data-board-drop]');
      const board = over ? parseInt(over.getAttribute('data-board-drop') || '', 10) : NaN;
      this.dropBoard = Number.isNaN(board) ? null : board;
      this.$store.commit('setTagDropBoard', this.dropBoard);
      this.dropIndex = this.dropBoard === null ? this.indexAt(e.clientY) : null;
    },
    // Index of the row the pointer is above (rows.length = after the last one).
    indexAt(y: number): number {
      const rows = (this.$refs.list as HTMLElement).querySelectorAll('[data-tag-row]');
      for (let i = 0; i < rows.length; i++) {
        const r = rows[i].getBoundingClientRect();
        if (y < r.top + r.height / 2) return i;
      }
      return rows.length;
    },
    onDragEnd() {
      const tag = this.dragging;
      const to = this.dropIndex;
      const board = this.dropBoard;
      this.stopDrag();
      if (tag !== null && board !== null) {
        this.moveTag(tag, board);
        return;
      }
      if (tag === null || to === null) return;
      const order = this.tags.map(t => t.tag);
      const from = order.indexOf(tag);
      order.splice(from, 1);
      order.splice(to > from ? to - 1 : to, 0, tag);
      if (order.some((t, i) => t !== this.tags[i].tag)) this.$store.dispatch('reorderTags', order);
    },
    stopDrag() {
      window.removeEventListener('mousemove', this.onDragMove);
      window.removeEventListener('mouseup', this.onDragEnd);
      this.pressed = null;
      this.dropIndex = null;
      this.dropBoard = null;
      this.$store.commit('setTagDropBoard', null);
      // keep `dragging` set until after the trailing click event so select() can ignore it
      setTimeout(() => (this.dragging = null), 0);
    },

    // --- resize ---
    startResize() {
      window.addEventListener('mousemove', this.onResize);
      window.addEventListener('mouseup', this.stopResize);
    },
    onResize(e: MouseEvent) {
      const left = (this.$refs.nav as HTMLElement).getBoundingClientRect().left;
      applyWidth(Math.round(Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, e.clientX - left))));
    },
    stopResize() {
      window.removeEventListener('mousemove', this.onResize);
      window.removeEventListener('mouseup', this.stopResize);
      try {
        const w = parseInt(document.documentElement.style.getPropertyValue('--tags-w'), 10);
        if (w) localStorage.setItem(WIDTH_KEY, String(w));
      } catch {
        // width just won't persist
      }
    },
  },
});
</script>

<style scoped lang="scss">
.vx-context {
  position: fixed;
  z-index: 100;
  min-width: 190px;
  padding: 6px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: 0 12px 32px oklch(0 0 0 / 0.45);

  &__label {
    padding: 8px 10px 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  &__boards {
    max-height: 200px;
    overflow-y: auto;
  }

  &__name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

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
    cursor: pointer;

    &:hover {
      background: var(--surface-3);
    }
  }
}

.vx-split {
  &__hint {
    margin: 0 0 14px;
    color: var(--text-dim);
    font-size: 13px;
    line-height: 1.45;
  }
  &__label {
    display: block;
    margin-bottom: 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-dim);
  }
}

.vx-pop-enter-active {
  transition: opacity 0.1s var(--ease), transform 0.1s var(--ease);
}
.vx-pop-enter {
  opacity: 0;
  transform: scale(0.96);
}

.vx-tags {
  position: relative;
  display: flex;
  flex-direction: column;
  min-height: 0;
  padding: 18px 12px 14px 6px;

  &__head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 6px 12px 14px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  &__add {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: none;
    border-radius: 50%;
    background: none;
    color: var(--text-dim);
    cursor: pointer;
    transition: background 0.12s var(--ease), color 0.12s var(--ease);

    &:hover {
      background: var(--surface-2);
      color: var(--accent);
    }
  }

  &__list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow-y: auto;
    min-height: 0;
  }

  &__row {
    position: relative;
    display: flex;
    align-items: center;

    &--dragging {
      opacity: 0.45;
    }
    // drop indicator line
    &--drop-before::before,
    &--drop-after::after {
      content: '';
      position: absolute;
      left: 8px;
      right: 8px;
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

  &__item {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 12px 14px;
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-dim);
    font-size: 16px;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
    transition: background 0.12s var(--ease), color 0.12s var(--ease);

    &:hover {
      background: var(--surface-2);
      color: var(--text);
    }
    &--active {
      background: var(--accent-soft);
      color: var(--accent);
    }
  }

  &__name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &__count {
    flex: none;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    opacity: 0.75;
  }

  // edit/delete replace the count while the row is hovered
  &__actions {
    position: absolute;
    right: 4px;
    display: none;
    align-items: center;
    gap: 2px;
    padding-left: 6px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);

    &--open {
      display: flex;
    }
  }
  &__row:hover &__actions {
    display: flex;
  }

  &__act {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-dim);
    cursor: pointer;

    &:hover {
      color: var(--text);
      background: var(--surface-3);
    }
    &--danger:hover {
      color: var(--danger);
    }
  }

  &__confirm {
    font-size: 13px;
    color: var(--danger);
  }

  &__input {
    width: 100%;
    height: 44px;
    padding: 0 14px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font-size: 16px;
    outline: none;

    &::placeholder {
      color: var(--text-dim);
    }
  }

  // drag handle on the right edge to change the rail width
  &__resize {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: 8px;
    cursor: col-resize;

    &:hover,
    &:active {
      background: linear-gradient(to right, transparent 3px, var(--accent-soft) 3px, var(--accent-soft) 5px, transparent 5px);
    }
  }
}
</style>
