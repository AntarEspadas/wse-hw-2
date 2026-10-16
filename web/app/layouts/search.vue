<template>
  <v-container class="py-16" :class="{ 'fill-height': isHome }">
    <v-row justify="center">
      <v-col cols="12" lg="8" md="10">
        <v-form @submit.prevent="onSubmit">
          <v-row>
            <v-col cols="12" sm="8">
              <v-text-field
                v-model="query"
                hide-details
                label="Search"
                prepend-inner-icon="mdi-magnify"
                type="search"
                @keydown.enter.prevent="onSubmit"
              />
            </v-col>

            <v-col cols="12" sm="4">
              <v-select
                v-model="queryType"
                hide-details
                :items="queryTypes"
                label="Query mode"
              />
            </v-col>
          </v-row>
        </v-form>

        <slot />
      </v-col>
    </v-row>
  </v-container>
</template>

<script lang="ts" setup>
  const route = useRoute()
  const router = useRouter()

  const isHome = computed(() => route.path === '/')
  const query = ref('')
  const queryType = ref('con')

  const queryTypes = [
    { title: 'Conjunctive', value: 'con' },
    { title: 'Disjunctive', value: 'dis' },
  ]

  watch(() => [route.query.q, route.query.t], ([q, t]) => {
    query.value = typeof q === 'string' ? q : ''
    queryType.value = t === 'dis' ? 'dis' : 'con'
  }, { immediate: true })

  function onSubmit() {
    const searchQuery = query.value.trim()
    if (!searchQuery) {
      return
    }

    router.push({ path: '/search', query: { t: queryType.value, q: searchQuery } })
  }
</script>
