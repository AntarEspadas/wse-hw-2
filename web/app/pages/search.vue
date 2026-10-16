<template>
  <div>

    <div v-if="pending" />

    <div v-for="(result, i) in docResults" v-else :key="i">
      <p>{{ result.doc_id }}</p>
      <p>{{ result.score }}</p>

      <p>
        <template v-for="entry in result.snippet">
          <strong v-if="entry.highlight">{{ entry.token }}</strong>

          <template v-else>
            {{ entry.token }}
          </template>
          {{ " " }}
        </template>
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
  import type { SearchResults } from '~/types';


  definePageMeta({
    layout: "search"
  })

  const route = useRoute()

  console.log(route.query.t)
  const { data , pending } = useFetch<SearchResults>("/api/search", {query: () => ({ t: route.query.t ?? "con", q: route.query.q ?? "" })})

  const docResults = computed(() => data.value?.results.map(x => ({...x, snippet: adaptSnippet(x.snippet, x.term_indices)})))

  function adaptSnippet(snippet: string, termIndices: number[]) {
    const tokens = snippet.split(/\s+/)

    const result = tokens.map(token => ({token, highlight: false}))

    for (const index of termIndices) {
      result[index]!.highlight = true
    }

    return result
  }
</script>