export function findUserEmail(db: { query: (q: string) => unknown }) {
  return db.query(`SELECT users.email FROM users WHERE users.id = $1`);
}
