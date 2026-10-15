<template>
  <div>

    <div v-if="pending" />

    <div v-for="(result, i) in data.results" v-else :key="i">
      <p>{{ result.score }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">

  definePageMeta({
    layout: "search"
  })

  const route = useRoute()

  console.log(route.query.t)
  const { data , pending } = useFetch<any>("/api/search", {query: () => ({ t: route.query.t ?? "con", q: route.query.q ?? "" })})
</script>