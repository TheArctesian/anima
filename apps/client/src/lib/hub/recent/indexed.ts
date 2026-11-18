import type { Copy, CopyStore } from './copy';

const DATABASE = 'anima-recent';
const COPIES = 'copies';
const BY_AGE = 'at';

function settled<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error('The copy store refused the request.'));
  });
}

function finished(transaction: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    transaction.oncomplete = () => resolve();
    transaction.onerror = () => reject(transaction.error ?? new Error('The copy store refused the change.'));
    transaction.onabort = () => reject(transaction.error ?? new Error('The copy store stopped the change.'));
  });
}

function open(factory: IDBFactory): Promise<IDBDatabase> {
  const request = factory.open(DATABASE, 1);
  request.onupgradeneeded = () => {
    request.result.createObjectStore(COPIES, { keyPath: 'key' }).createIndex(BY_AGE, 'at');
  };
  return settled(request);
}

/** Deletes the oldest copies until `most` stay; runs in the transaction of the write, so the two succeed or fail together. */
function dropOldest(copies: IDBObjectStore, most: number): void {
  const count = copies.count();
  count.onsuccess = () => {
    let extra = count.result - most;
    if (extra <= 0) return;
    const oldest = copies.index(BY_AGE).openKeyCursor();
    oldest.onsuccess = () => {
      const cursor = oldest.result;
      if (cursor === null || extra <= 0) return;
      copies.delete(cursor.primaryKey);
      extra -= 1;
      cursor.continue();
    };
  };
}

/** The copy store of the application's web view: one IndexedDB database on this computer (FND-007). */
export function indexedCopyStore(factory: IDBFactory): CopyStore {
  let database: Promise<IDBDatabase> | undefined;
  const change = async (work: (copies: IDBObjectStore) => void): Promise<void> => {
    database ??= open(factory);
    const transaction = (await database).transaction(COPIES, 'readwrite');
    work(transaction.objectStore(COPIES));
    await finished(transaction);
  };
  return {
    async read(key) {
      database ??= open(factory);
      const copies = (await database).transaction(COPIES, 'readonly').objectStore(COPIES);
      return settled(copies.get(key) as IDBRequest<Copy | undefined>);
    },
    write: (copy, most) =>
      change((copies) => {
        copies.put(copy);
        dropOldest(copies, most);
      }),
    remove: (key) => change((copies) => void copies.delete(key)),
    clear: () => change((copies) => void copies.clear()),
  };
}
