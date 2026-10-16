<template>
  <div>

    <v-select
      v-model="queryType"
      :items="queryTypes"
      label="Query mode"
    />

    <v-form @submit.prevent="onSubmit">

      <v-text-field v-model="query" label="Search" />
    </v-form>

    <slot />
  </div>
</template>

<script lang="ts" setup>
  const router = useRouter()

  const query = ref("")

  const queryTypes = [{title: "Conjunctive", value: "con"}, {title: "Disjunctive", value: "dis"}]
  const queryType = ref(queryTypes[0])

  function onSubmit() {
    console.log("Query?")
    if (query.value.trim() === "") {
      return
    }
    router.push({path: "/search", query: {t: queryType.value?.value, q: query.value}})
  }
</script>