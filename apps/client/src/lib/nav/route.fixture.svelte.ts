// Test support: the current route as Svelte state, standing in for `page` from `$app/state`.
export const route = $state({ pathname: '/devices' });
