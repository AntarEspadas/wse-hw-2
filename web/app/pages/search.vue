<template>
  <section :aria-busy="pending" aria-label="Search results" aria-live="polite" class="mt-6">
    <v-progress-linear v-if="pending" aria-label="Loading results" indeterminate />

    <v-alert v-else-if="error" text="Search unavailable. Please try again." type="error" variant="tonal" />

    <v-list v-else-if="docResults.length > 0" class="pa-0">
      <template v-for="(result, index) in docResults" :key="result.doc_id">
        <v-divider v-if="index" />

        <v-list-item class="px-0 py-4">
          <div class="d-flex flex-wrap align-baseline justify-space-between ga-2 mb-2">
            <h2 class="text-subtitle-1">Document {{ result.doc_id }}</h2>

            <span class="text-caption text-medium-emphasis">Score: {{ result.score }}</span>
          </div>

          <p class="text-body-2">
            <template v-for="(entry, tokenIndex) in result.snippet" :key="tokenIndex">
              <strong v-if="entry.highlight">{{ entry.token }}</strong>

              <template v-else>{{ entry.token }}</template>
              {{ ' ' }}
            </template>
          </p>
        </v-list-item>
      </template>
    </v-list>

    <p v-else class="text-body-2 text-medium-emphasis">No results found.</p>
  </section>
</template>

<script setup lang="ts">
  import type { SearchResults } from '~/types'

  definePageMeta({
    layout: 'search',
  })

  const route = useRoute()

  const { data, pending, error } = useFetch<SearchResults>('/api/search', {
    query: () => ({ t: route.query.t ?? 'con', q: route.query.q ?? '' }),
  })

  const docResults = computed(() => data.value?.results.map(result => ({
    ...result,
    snippet: adaptSnippet(result.snippet, result.term_indices),
  })) ?? [])

  function adaptSnippet(snippet: string, termIndices: number[]) {
    const tokens = snippet.split(/\s+/)

    const result = tokens.map(token => ({ token, highlight: false }))

    for (const index of termIndices) {
      if (result[index]) {
        result[index].highlight = true
      }
    }

    return result
  }
</script>
