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

/** Opens the database; `lost` hears when the web view closes the connection, so that the next call opens it again. */
async function open(factory: IDBFactory, lost: () => void): Promise<IDBDatabase> {
  const request = factory.open(DATABASE, 1);
  request.onupgradeneeded = () => {
    request.result.createObjectStore(COPIES, { keyPath: 'key' }).createIndex(BY_AGE, 'at');
  };
  const database = await settled(request);
  database.onclose = lost;
  database.onversionchange = () => {
    database.close();
    lost();
  };
  return database;
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
  const forget = () => {
    database = undefined;
  };
  /** A transaction on the open database. A database that cannot open or lost its connection is opened again by the next call. */
  const begin = async (mode: IDBTransactionMode): Promise<IDBTransaction> => {
    database ??= open(factory, forget);
    try {
      return (await database).transaction(COPIES, mode);
    } catch (reason) {
      forget();
      throw reason;
    }
  };
  const change = async (work: (copies: IDBObjectStore) => void): Promise<void> => {
    const transaction = await begin('readwrite');
    work(transaction.objectStore(COPIES));
    await finished(transaction);
  };
  return {
    async read(key) {
      const copies = (await begin('readonly')).objectStore(COPIES);
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
