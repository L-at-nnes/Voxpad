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
  </nav>
</template>

<script lang="ts">
import Vue from 'vue';
import VxIcon from '@/components/VxIcon.vue';
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
  components: { VxIcon },
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
  },
  created() {
    try {
      const saved = parseInt(localStorage.getItem(WIDTH_KEY) || '', 10);
      if (saved >= MIN_WIDTH && saved <= MAX_WIDTH) applyWidth(saved);
    } catch {
      // default width from the stylesheet
    }
  },
  beforeDestroy() {
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
      this.dropIndex = this.indexAt(e.clientY);
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
      this.stopDrag();
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
