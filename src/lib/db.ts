import Database, { type QueryResult } from "@tauri-apps/plugin-sql";

const DB_PATH = "sqlite:store.db";

let dbPromise: Promise<Database> | null = null;

/** Lazily load (and cache) the SQLite connection. Migrations run on first load. */
export function getDb(): Promise<Database> {
  if (!dbPromise) {
    dbPromise = Database.load(DB_PATH);
  }
  return dbPromise;
}

/** Run a SELECT query and return all matching rows. */
export async function select<T>(query: string, params: unknown[] = []): Promise<T[]> {
  const client = await getDb();
  return client.select<T[]>(query, params);
}

/** Run a SELECT query and return the first row, or undefined. */
export async function selectOne<T>(query: string, params: unknown[] = []): Promise<T | undefined> {
  const rows = await select<T>(query, params);
  return rows[0];
}

/** Run an INSERT/UPDATE/DELETE query. */
export function execute(query: string, params: unknown[] = []): Promise<QueryResult> {
  return getDb().then((client) => client.execute(query, params));
}

/** Insert a row and return its auto-increment id. */
export async function insert(query: string, params: unknown[] = []): Promise<number> {
  const result = await execute(query, params);
  return result.lastInsertId ?? 0;
}

/** Close the connection pool (e.g. on app exit or restore). */
export async function closeDb(): Promise<void> {
  const current = dbPromise;
  dbPromise = null;
  if (current) {
    await current.then((client) => client.close());
  }
}
