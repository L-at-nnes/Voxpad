<template>
  <VxModal :value="!!sound" :title="title" :width="440" @input="close">
    <div v-if="sound" class="vx-tagdlg">
      <p v-if="!tags.length" class="vx-tagdlg__hint">{{ $t('tags.noneYet') }}</p>
      <div v-else class="vx-tagdlg__chips">
        <button
          v-for="t in tags"
          :key="t.tag"
          class="vx-tagdlg__chip"
          :class="{ 'vx-tagdlg__chip--on': has(t.tag) }"
          @click="toggle(t.tag)"
        >
          <VxIcon :name="has(t.tag) ? 'check' : 'tag'" :size="13" />
          {{ t.tag }}
        </button>
      </div>

      <div class="vx-tagdlg__new">
        <input
          ref="input"
          v-model="draft"
          class="vx-input"
          type="text"
          :placeholder="$t('tags.newTag')"
          maxlength="32"
          spellcheck="false"
          @keydown.enter="create"
        />
        <button class="vx-btn vx-btn--accent" :disabled="!draft.trim()" @click="create">
          <VxIcon name="plus" :size="15" /> {{ $t('tags.create') }}
        </button>
      </div>
    </div>
  </VxModal>
</template>

<script lang="ts">
import Vue from 'vue';
import { Sound } from '@/types';
import VxModal from '@/components/VxModal.vue';
import VxIcon from '@/components/VxIcon.vue';

export default Vue.extend({
  name: 'TagDialog',
  components: { VxModal, VxIcon },
  data: () => ({ draft: '' }),
  computed: {
    sound(): Sound | null {
      return this.$store.getters.tagDialogSound;
    },
    tags(): { tag: string; count: number }[] {
      return this.$store.getters.tagCounts;
    },
    title(): string {
      return this.sound ? `${this.$t('tags.manageTitle')} — ${this.sound.name}` : '';
    },
  },
  watch: {
    sound(s: Sound | null, prev: Sound | null) {
      if (s && !prev) {
        this.draft = '';
        this.$nextTick(() => (this.$refs.input as HTMLInputElement | undefined)?.focus());
      }
    },
  },
  methods: {
    close() {
      this.$store.commit('setTagDialogSound', null);
    },
    has(tag: string): boolean {
      return !!this.sound && this.sound.tags.includes(tag);
    },
    setTags(tags: string[]) {
      if (this.sound) this.$store.dispatch('setSoundTags', { sound: this.sound, tags });
    },
    toggle(tag: string) {
      if (!this.sound) return;
      this.setTags(this.has(tag) ? this.sound.tags.filter(t => t !== tag) : [...this.sound.tags, tag]);
    },
    create() {
      const draft = this.draft.trim();
      if (!draft || !this.sound) return;
      // Reuse an existing tag's spelling so "meme"/"Meme" don't become two filters.
      const known = this.tags.find(t => t.tag.toLowerCase() === draft.toLowerCase());
      const tag = known ? known.tag : draft;
      if (!this.has(tag)) this.setTags([...this.sound.tags, tag]);
      this.draft = '';
    },
  },
});
</script>

<style scoped lang="scss">
.vx-tagdlg {
  display: flex;
  flex-direction: column;
  gap: 16px;

  &__hint {
    margin: 0;
    color: var(--text-dim);
  }

  &__chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    max-height: 240px;
    overflow-y: auto;
  }

  &__chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 99px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-dim);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.12s var(--ease);

    &:hover {
      color: var(--text);
      background: var(--surface-2);
    }
    &--on {
      color: var(--accent);
      border-color: var(--accent-soft);
      background: var(--accent-soft);
    }
  }

  &__new {
    display: flex;
    gap: 8px;

    .vx-input {
      flex: 1;
      min-width: 0;
    }
  }
}
</style>
